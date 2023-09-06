// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bdk_electrum::bdk_chain::local_chain::{CannotConnectError, CheckPoint};
use bdk_electrum::electrum_client::{
    Client as ElectrumClient, Config as ElectrumConfig, Socks5Config,
};
use bdk_electrum::ElectrumExt;
use coinstr_core::bdk::chain::{ConfirmationTime, ConfirmationTimeAnchor, TxGraph};
use coinstr_core::bdk::wallet::{AddressIndex, AddressInfo, Balance, Update};
use coinstr_core::bdk::{FeeRate, KeychainKind, LocalUtxo, Wallet};
use coinstr_core::bitcoin::address::NetworkUnchecked;
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Address, OutPoint, Script, ScriptBuf, Transaction, Txid};
use coinstr_core::reserves::ProofOfReserves;
use coinstr_core::{Amount, Policy, Proposal};
use thiserror::Error;
use tokio::sync::RwLock;

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
    Address(#[from] coinstr_core::bitcoin::address::Error),
    #[error(transparent)]
    Electrum(#[from] bdk_electrum::electrum_client::Error),
    #[error(transparent)]
    CannotConnect(#[from] CannotConnectError),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("impossible to read wallet")]
    ImpossibleToReadWallet,
    #[error("not found")]
    NotFound,
    #[error("already syncing")]
    AlreadySyncing,
    #[error("impossible to insert tx: {0}")]
    InsertTx(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionDetails {
    pub transaction: Transaction,
    pub received: u64,
    pub sent: u64,
    pub fee: Option<u64>,
    pub confirmation_time: ConfirmationTime,
}

impl Deref for TransactionDetails {
    type Target = Transaction;
    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
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

    pub async fn latest_checkpoint(&self) -> Option<CheckPoint> {
        self.wallet.read().await.latest_checkpoint().clone()
    }

    pub async fn graph(&self) -> TxGraph<ConfirmationTimeAnchor> {
        self.wallet.read().await.as_ref().clone()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn spks(
        &self,
    ) -> BTreeMap<KeychainKind, impl Iterator<Item = (u32, ScriptBuf)> + Clone> {
        self.wallet.read().await.spks_of_all_keychains()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn spks_of_keychain(
        &self,
        keychain: KeychainKind,
    ) -> impl Iterator<Item = (u32, ScriptBuf)> + Clone {
        self.wallet.read().await.spks_of_keychain(keychain)
    }

    pub async fn insert_tx(
        &self,
        tx: Transaction,
        position: ConfirmationTime,
    ) -> Result<bool, Error> {
        let mut wallet = self.wallet.write().await;
        let res = wallet
            .insert_tx(tx, position)
            .map_err(|e| Error::InsertTx(format!("{e:?}")))?;
        wallet.commit()?;
        Ok(res)
    }

    pub async fn is_mine(&self, script: &Script) -> bool {
        self.wallet.read().await.is_mine(script)
    }

    pub async fn get_balance(&self) -> Balance {
        self.wallet.read().await.get_balance()
    }

    pub async fn get_address(&self, index: AddressIndex) -> AddressInfo {
        let mut wallet = self.wallet.write().await;
        wallet.get_address(index)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_addresses(&self) -> Result<Vec<Address<NetworkUnchecked>>, Error> {
        // Get spks
        let spks = self.spks_of_keychain(KeychainKind::External).await;

        // Get last unused address
        let last_unused = self.get_address(AddressIndex::LastUnused).await;

        // Get network
        let wallet = self.wallet.read().await;
        let network = wallet.network();
        drop(wallet);

        let mut addresses: Vec<Address<NetworkUnchecked>> = Vec::new();
        let mut counter: Option<u8> = None;

        for (_index, script) in spks {
            let addr: Address = Address::from_script(&script, network)?;
            let addr_unchecked: Address<NetworkUnchecked> =
                Address::new(network, addr.payload.clone());
            addresses.push(addr_unchecked);

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
    pub async fn get_addresses_balances(&self) -> HashMap<ScriptBuf, u64> {
        let mut map: HashMap<ScriptBuf, u64> = HashMap::new();

        for utxo in self.wallet.read().await.list_unspent() {
            map.entry(utxo.txout.script_pubkey)
                .and_modify(|amount| *amount += utxo.txout.value)
                .or_insert(utxo.txout.value);
        }

        map
    }

    pub async fn get_txs(&self) -> Vec<TransactionDetails> {
        let wallet = self.wallet.read().await;
        let mut txs = Vec::new();
        for canonical_tx in wallet.transactions() {
            let tx: &Transaction = canonical_tx.tx_node.tx;
            let confirmation_time: ConfirmationTime = canonical_tx.chain_position.cloned().into();
            let (sent, received) = wallet.sent_and_received(tx);
            let fee: Option<u64> = wallet.calculate_fee(tx).ok();
            txs.push(TransactionDetails {
                transaction: tx.clone(),
                received,
                sent,
                fee,
                confirmation_time,
            })
        }
        txs
    }

    pub async fn get_tx(&self, txid: Txid) -> Result<TransactionDetails, Error> {
        let wallet = self.wallet.read().await;
        let canonical_tx = wallet.get_tx(txid).ok_or(Error::NotFound)?;
        let tx: &Transaction = canonical_tx.tx_node.tx;
        let confirmation_time: ConfirmationTime = canonical_tx.chain_position.cloned().into();
        let (sent, received) = wallet.sent_and_received(tx);
        let fee: Option<u64> = wallet.calculate_fee(tx).ok();
        Ok(TransactionDetails {
            transaction: tx.clone(),
            received,
            sent,
            fee,
            confirmation_time,
        })
    }

    pub async fn get_utxos(&self) -> Vec<LocalUtxo> {
        let wallet = self.wallet.read().await;
        wallet.list_unspent().collect()
    }

    pub async fn sync<S>(&self, endpoint: S, proxy: Option<SocketAddr>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        if !self.is_syncing() {
            self.set_syncing(true);

            let endpoint: String = endpoint.into();
            let prev_tip: Option<CheckPoint> = self.latest_checkpoint().await;
            let keychain_spks = self.spks().await;
            let graph: TxGraph<ConfirmationTimeAnchor> = self.graph().await;

            tracing::info!("Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}");
            let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
            let config: ElectrumConfig = ElectrumConfig::builder().socks5(proxy).build();
            let client: ElectrumClient = ElectrumClient::from_config(&endpoint, config)?;

            let electrum_update =
                client.scan(prev_tip, keychain_spks, None, None, STOP_GAP, BATCH_SIZE)?;
            let missing: Vec<Txid> = electrum_update.missing_full_txs(&graph);
            let update = electrum_update.finalize_as_confirmation_time(&client, None, missing)?;

            self.apply_update(update).await?;

            self.set_syncing(false);

            Ok(())
        } else {
            Err(Error::AlreadySyncing)
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn apply_update(&self, update: Update) -> Result<(), Error> {
        let mut wallet = self.wallet.write().await;
        wallet.apply_update(update)?;
        wallet.commit()?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn spend<S>(
        &self,
        address: Address<NetworkUnchecked>,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        frozen_utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        let mut wallet = self.wallet.write().await;
        let proposal = self.policy.spend(
            &mut wallet,
            address,
            amount,
            description,
            fee_rate,
            utxos,
            frozen_utxos,
            policy_path,
        )?;
        Ok(proposal)
    }

    pub async fn proof_of_reserve<S>(&self, message: S) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        let mut wallet = self.wallet.write().await;
        let proposal = self.policy.proof_of_reserve(&mut wallet, message)?;
        Ok(proposal)
    }

    pub async fn verify_proof<S>(
        &self,
        psbt: &PartiallySignedTransaction,
        message: S,
    ) -> Result<u64, Error>
    where
        S: Into<String>,
    {
        Ok(self.wallet.read().await.verify_proof(psbt, message, None)?)
    }
}
