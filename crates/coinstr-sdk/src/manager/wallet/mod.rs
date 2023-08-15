// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bdk_electrum::electrum_client::{
    Client as ElectrumClient, Config as ElectrumConfig, Socks5Config,
};
use bdk_electrum::ElectrumExt;
use coinstr_core::bdk::chain::keychain::LocalUpdate;
use coinstr_core::bdk::chain::local_chain::UpdateNotConnectedError;
use coinstr_core::bdk::chain::{ConfirmationTimeAnchor, TxGraph};
use coinstr_core::bdk::wallet::{AddressIndex, AddressInfo, Balance};
use coinstr_core::bdk::{FeeRate, KeychainKind, LocalUtxo, TransactionDetails, Wallet};
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Address, BlockHash, OutPoint, Script, Txid};
use coinstr_core::reserves::ProofOfReserves;
use coinstr_core::{Amount, Policy, Proposal};
use parking_lot::RwLock;
use thiserror::Error;

mod storage;

pub use self::storage::{CoinstrWalletStorage, Error as StorageError};

const STOP_GAP: usize = 50;
const BATCH_SIZE: usize = 5;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Policy(#[from] coinstr_core::policy::Error),
    #[error(transparent)]
    Proof(#[from] coinstr_core::reserves::ProofError),
    #[error(transparent)]
    Address(#[from] coinstr_core::bitcoin::util::address::Error),
    #[error(transparent)]
    Electrum(#[from] bdk_electrum::electrum_client::Error),
    #[error(transparent)]
    UpdateNotConnected(#[from] UpdateNotConnectedError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("impossible to read wallet")]
    ImpossibleToReadWallet,
    #[error("not found")]
    NotFound,
    #[error("already syncing")]
    AlreadySyncing,
}

#[derive(Debug, Clone)]
pub struct CoinstrWallet {
    policy: Policy,
    wallet: Arc<RwLock<Wallet<CoinstrWalletStorage>>>,
    syncing: Arc<AtomicBool>,
}

impl CoinstrWallet {
    pub fn new(policy: Policy, wallet: Wallet<CoinstrWalletStorage>) -> Self {
        Self {
            policy,
            wallet: Arc::new(RwLock::new(wallet)),
            syncing: Arc::new(AtomicBool::new(false)),
        }
    }

    fn is_syncing(&self) -> bool {
        self.syncing.load(Ordering::SeqCst)
    }

    fn set_syncing(&self, syncing: bool) {
        let _ = self
            .syncing
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(syncing));
    }

    pub fn checkpoints(&self) -> BTreeMap<u32, BlockHash> {
        self.wallet.read().checkpoints().clone()
    }

    pub fn graph(&self) -> TxGraph<ConfirmationTimeAnchor> {
        self.wallet.read().as_ref().clone()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn spks(&self) -> BTreeMap<KeychainKind, impl Iterator<Item = (u32, Script)> + Clone> {
        self.wallet.read().spks_of_all_keychains()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn spks_of_keychain(
        &self,
        keychain: KeychainKind,
    ) -> impl Iterator<Item = (u32, Script)> + Clone {
        self.wallet.read().spks_of_keychain(keychain)
    }

    pub fn is_mine(&self, script: &Script) -> bool {
        self.wallet.read().is_mine(script)
    }

    pub fn get_balance(&self) -> Balance {
        self.wallet.read().get_balance()
    }

    pub fn get_address(&self, index: AddressIndex) -> AddressInfo {
        let mut wallet = self.wallet.write();
        wallet.get_address(index)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_addresses(&self) -> Result<Vec<Address>, Error> {
        // Get spks
        let spks = self.spks_of_keychain(KeychainKind::External);

        // Get last unused address
        let last_unused = self.get_address(AddressIndex::LastUnused);

        // Get network
        let wallet = self.wallet.read();
        let network = wallet.network();
        drop(wallet);

        let mut addresses: Vec<Address> = Vec::new();
        let mut counter: Option<u8> = None;

        for (_index, script) in spks {
            let addr = Address::from_script(&script, network)?;
            addresses.push(addr.clone());
            if addr == last_unused.address {
                counter = Some(0);
            }
            if let Some(counter) = counter.as_mut() {
                *counter += 1;

                if *counter >= 20 {
                    break;
                }
            }
        }

        Ok(addresses)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_addresses_balances(&self) -> HashMap<Script, u64> {
        let mut map: HashMap<Script, u64> = HashMap::new();

        for utxo in self.wallet.read().list_unspent() {
            map.entry(utxo.txout.script_pubkey)
                .and_modify(|amount| *amount += utxo.txout.value)
                .or_insert(utxo.txout.value);
        }

        map
    }

    pub fn get_txs(&self) -> Vec<TransactionDetails> {
        let wallet = self.wallet.read();
        wallet
            .transactions()
            .filter_map(|t| wallet.get_tx(t.node.txid, true))
            .collect()
    }

    pub fn get_tx(&self, txid: Txid) -> Result<TransactionDetails, Error> {
        self.wallet.read().get_tx(txid, true).ok_or(Error::NotFound)
    }

    pub fn get_utxos(&self) -> Vec<LocalUtxo> {
        let wallet = self.wallet.read();
        wallet.list_unspent().collect()
    }

    pub fn sync<S>(&self, endpoint: S, proxy: Option<SocketAddr>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        if !self.is_syncing() {
            self.set_syncing(true);

            let endpoint: String = endpoint.into();
            let local_chain = self.checkpoints();
            let keychain_spks = self.spks();
            let graph = self.graph();

            tracing::info!("Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}");
            let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
            let config = ElectrumConfig::builder().socks5(proxy)?.build();
            let client = ElectrumClient::from_config(&endpoint, config)?;

            let electrum_update = client.scan(
                &local_chain,
                keychain_spks,
                None,
                None,
                STOP_GAP,
                BATCH_SIZE,
            )?;
            let missing = electrum_update.missing_full_txs(&graph);
            let update = electrum_update.finalize_as_confirmation_time(&client, None, missing)?;

            self.apply_update(update)?;

            self.set_syncing(false);

            Ok(())
        } else {
            Err(Error::AlreadySyncing)
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn apply_update(
        &self,
        update: LocalUpdate<KeychainKind, ConfirmationTimeAnchor>,
    ) -> Result<(), Error> {
        let mut wallet = self.wallet.write();
        wallet.apply_update(update)?;
        wallet.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spend<S>(
        &self,
        address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        let mut wallet = self.wallet.write();
        let proposal = self.policy.spend(
            &mut wallet,
            address,
            amount,
            description,
            fee_rate,
            utxos,
            policy_path,
        )?;
        Ok(proposal)
    }

    pub fn proof_of_reserve<S>(&self, message: S) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        let mut wallet = self.wallet.write();
        let proposal = self.policy.proof_of_reserve(&mut wallet, message)?;
        Ok(proposal)
    }

    pub fn verify_proof<S>(
        &self,
        psbt: &PartiallySignedTransaction,
        message: S,
    ) -> Result<u64, Error>
    where
        S: Into<String>,
    {
        Ok(self.wallet.read().verify_proof(psbt, message, None)?)
    }
}
