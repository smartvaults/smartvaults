// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bdk::bitcoin::{Address, BlockHash, OutPoint, Script, Txid};
use bdk::chain::keychain::LocalUpdate;
use bdk::chain::local_chain::UpdateNotConnectedError;
use bdk::chain::{ConfirmationTimeAnchor, TxGraph};
use bdk::wallet::{AddressIndex, AddressInfo, Balance};
use bdk::{FeeRate, KeychainKind, LocalUtxo, TransactionDetails, Wallet};
use bdk_electrum::electrum_client::{
    Client as ElectrumClient, Config as ElectrumConfig, Socks5Config,
};
use bdk_electrum::ElectrumExt;
use coinstr_core::{Amount, Policy, Proposal};
use parking_lot::RwLock;
use thiserror::Error;

use crate::storage::CoinstrWalletStorage;

const STOP_GAP: usize = 50;
const BATCH_SIZE: usize = 5;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Policy(#[from] coinstr_core::policy::Error),
    #[error(transparent)]
    Electrum(#[from] bdk_electrum::electrum_client::Error),
    #[error(transparent)]
    UpdateNotConnected(#[from] UpdateNotConnectedError),
    #[error(transparent)]
    Storage(#[from] crate::storage::Error),
    #[error("impossible to read wallet")]
    ImpossibleToReadWallet,
    #[error("not found")]
    NotFound,
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

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn checkpoints(&self) -> BTreeMap<u32, BlockHash> {
        self.wallet.read().checkpoints().clone()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn graph(&self) -> TxGraph<ConfirmationTimeAnchor> {
        self.wallet.read().as_ref().clone()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn spks(&self) -> BTreeMap<KeychainKind, impl Iterator<Item = (u32, Script)> + Clone> {
        self.wallet.read().spks_of_all_keychains()
    }

    pub fn is_mine(&self, script: &Script) -> bool {
        self.wallet.read().is_mine(script)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_balance(&self) -> Balance {
        self.wallet.read().get_balance()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_address(&self, index: AddressIndex) -> AddressInfo {
        let mut wallet = self.wallet.write();
        wallet.get_address(index)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_addresses(&self) -> Vec<Address> {
        let mut wallet = self.wallet.write();

        let last_unused = wallet.get_address(AddressIndex::LastUnused);

        let mut addresses: Vec<Address> = Vec::new();

        for index in 0.. {
            let addr = wallet.get_address(AddressIndex::Peek(index));
            addresses.push(addr.address.clone());
            if addr == last_unused {
                for i in index + 1..index + 20 {
                    let addr = wallet.get_address(AddressIndex::Peek(i));
                    addresses.push(addr.address);
                }
                break;
            }
        }

        addresses
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

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_txs(&self) -> Vec<TransactionDetails> {
        let wallet = self.wallet.read();
        wallet
            .transactions()
            .filter_map(|t| wallet.get_tx(t.node.txid, true))
            .collect()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_tx(&self, txid: Txid) -> Result<TransactionDetails, Error> {
        self.wallet.read().get_tx(txid, true).ok_or(Error::NotFound)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_utxos(&self) -> Vec<LocalUtxo> {
        let wallet = self.wallet.read();
        wallet.list_unspent().collect()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn sync<S>(&self, endpoint: S, proxy: Option<SocketAddr>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        if !self.is_syncing() {
            tracing::info!("Syncing policy {}", "TODO");

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

            tracing::info!("Policy TODO synced")
        } else {
            tracing::warn!("Policy TODO is already syncing");
        }
        Ok(())
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
    #[tracing::instrument(skip_all, level = "trace")]
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
}
