// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::net::SocketAddr;
use std::ops::Deref;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering as AtomicOrdering};
use std::sync::Arc;

use bdk_electrum::electrum_client::{
    Client as ElectrumClient, Config as ElectrumConfig, Socks5Config,
};
use bdk_electrum::{ElectrumExt, ElectrumUpdate};
use nostr_sdk::{EventId, Timestamp};
use smartvaults_core::bdk::chain::keychain::KeychainTxOutIndex;
use smartvaults_core::bdk::chain::local_chain::{CannotConnectError, CheckPoint, LocalChain};
use smartvaults_core::bdk::chain::{
    BlockId, ConfirmationTime, ConfirmationTimeHeightAnchor, TxGraph,
};
use smartvaults_core::bdk::wallet::error::CreateTxError;
use smartvaults_core::bdk::wallet::{AddressIndex, AddressInfo, Balance, Update};
use smartvaults_core::bdk::{FeeRate, KeychainKind, LocalOutput, Wallet};
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::{Address, OutPoint, Script, ScriptBuf, Transaction, Txid};
use smartvaults_core::reserves::ProofOfReserves;
use smartvaults_core::{Amount, Policy, Proposal};
use thiserror::Error;
use tokio::sync::RwLock;

mod storage;

pub use self::storage::{Error as StorageError, SmartVaultsWalletStorage};
use crate::config::ElectrumEndpoint;
use crate::constants::WALLET_SYNC_INTERVAL;

const STOP_GAP: usize = 50;
const BATCH_SIZE: usize = 5;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Policy(#[from] smartvaults_core::policy::Error),
    #[error(transparent)]
    Proof(#[from] smartvaults_core::reserves::ProofError),
    #[error(transparent)]
    Address(#[from] smartvaults_core::bitcoin::address::Error),
    #[error(transparent)]
    Electrum(#[from] bdk_electrum::electrum_client::Error),
    #[error(transparent)]
    CannotConnect(#[from] CannotConnectError),
    #[error(transparent)]
    BdkCreateTx(#[from] CreateTxError<StorageError>),
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error("impossible to read wallet")]
    ImpossibleToReadWallet,
    #[error("not found")]
    NotFound,
    #[error("already synced")]
    AlreadySynced,
    #[error("already syncing")]
    AlreadySyncing,
    #[error("impossible to insert tx: {0}")]
    InsertTx(String),
}

#[derive(Debug, Clone, Copy)]
pub struct Fee {
    pub amount: Option<u64>,
    pub rate: Option<FeeRate>,
}

impl PartialEq for Fee {
    fn eq(&self, other: &Self) -> bool {
        self.amount.eq(&other.amount)
    }
}

impl Eq for Fee {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionDetails {
    pub transaction: Transaction,
    pub received: u64,
    pub sent: u64,
    pub fee: Fee,
    pub confirmation_time: ConfirmationTime,
}

impl PartialOrd for TransactionDetails {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TransactionDetails {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.confirmation_time != other.confirmation_time {
            let this: u32 = match self.confirmation_time {
                ConfirmationTime::Confirmed { height, .. } => height,
                ConfirmationTime::Unconfirmed { .. } => u32::MAX,
            };

            let that: u32 = match other.confirmation_time {
                ConfirmationTime::Confirmed { height, .. } => height,
                ConfirmationTime::Unconfirmed { .. } => u32::MAX,
            };

            this.cmp(&that).reverse()
        } else {
            self.total().cmp(&other.total()).reverse()
        }
    }
}

impl Deref for TransactionDetails {
    type Target = Transaction;

    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

impl TransactionDetails {
    pub fn total(&self) -> i64 {
        let received: i64 = self.received as i64;
        let sent: i64 = self.sent as i64;
        received.saturating_sub(sent)
    }
}

#[derive(Debug, Clone)]
pub struct SmartVaultsWallet {
    id: EventId,
    policy: Policy,
    wallet: Arc<RwLock<Wallet<SmartVaultsWalletStorage>>>,
    syncing: Arc<AtomicBool>,
    last_sync: Arc<AtomicU64>,
}

impl SmartVaultsWallet {
    pub fn new(
        policy_id: EventId,
        policy: Policy,
        wallet: Wallet<SmartVaultsWalletStorage>,
    ) -> Self {
        Self {
            id: policy_id,
            policy,
            wallet: Arc::new(RwLock::new(wallet)),
            syncing: Arc::new(AtomicBool::new(false)),
            last_sync: Arc::new(AtomicU64::new(0)),
        }
    }

    fn is_syncing(&self) -> bool {
        self.syncing.load(AtomicOrdering::SeqCst)
    }

    fn set_syncing(&self, syncing: bool) {
        let _ = self
            .syncing
            .fetch_update(AtomicOrdering::SeqCst, AtomicOrdering::SeqCst, |_| {
                Some(syncing)
            });
    }

    pub fn last_sync(&self) -> Timestamp {
        Timestamp::from(self.last_sync.load(AtomicOrdering::SeqCst))
    }

    fn update_last_sync(&self) {
        let _ = self
            .last_sync
            .fetch_update(AtomicOrdering::SeqCst, AtomicOrdering::SeqCst, |_| {
                Some(Timestamp::now().as_u64())
            });
    }

    pub async fn latest_checkpoint(&self) -> CheckPoint {
        self.wallet.read().await.latest_checkpoint()
    }

    pub async fn chain(&self) -> LocalChain {
        self.wallet.read().await.local_chain().clone()
    }

    pub async fn is_chain_empty(&self) -> bool {
        // Check if exists only genesis
        self.wallet.read().await.local_chain().blocks().len() == 1
    }

    pub async fn graph(&self) -> TxGraph<ConfirmationTimeHeightAnchor> {
        self.wallet.read().await.tx_graph().clone()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn spks(
        &self,
    ) -> BTreeMap<KeychainKind, impl Iterator<Item = (u32, ScriptBuf)> + Clone> {
        self.wallet.read().await.all_unbounded_spk_iters()
    }

    pub async fn spk_index(&self) -> KeychainTxOutIndex<KeychainKind> {
        let wallet = self.wallet.read().await;
        wallet.spk_index().clone()
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

    pub async fn get_address(&self, index: AddressIndex) -> Result<AddressInfo, Error> {
        let mut wallet = self.wallet.write().await;
        Ok(wallet.try_get_address(index)?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_addresses(&self) -> Result<Vec<Address<NetworkUnchecked>>, Error> {
        // Get last unused address
        let last_unused = self.get_address(AddressIndex::LastUnused).await?;

        // Get network and spks
        let wallet = self.wallet.read().await;
        let network = wallet.network();
        let spks = wallet.unbounded_spk_iter(KeychainKind::External);
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

    /// Get wallet TXs
    pub async fn txs(&self) -> BTreeSet<TransactionDetails> {
        let wallet = self.wallet.read().await;
        wallet
            .transactions()
            .map(|canonical_tx| {
                let tx: &Transaction = canonical_tx.tx_node.tx;
                let confirmation_time: ConfirmationTime =
                    canonical_tx.chain_position.cloned().into();
                let (sent, received) = wallet.sent_and_received(tx);
                TransactionDetails {
                    transaction: tx.clone(),
                    received,
                    sent,
                    fee: Fee {
                        amount: wallet.calculate_fee(tx).ok(),
                        rate: wallet.calculate_fee_rate(tx).ok(),
                    },
                    confirmation_time,
                }
            })
            .collect()
    }

    pub async fn get_tx(&self, txid: Txid) -> Result<TransactionDetails, Error> {
        let wallet = self.wallet.read().await;
        let canonical_tx = wallet.get_tx(txid).ok_or(Error::NotFound)?;
        let tx: &Transaction = canonical_tx.tx_node.tx;
        let confirmation_time: ConfirmationTime = canonical_tx.chain_position.cloned().into();
        let (sent, received) = wallet.sent_and_received(tx);
        Ok(TransactionDetails {
            transaction: tx.clone(),
            received,
            sent,
            fee: Fee {
                amount: wallet.calculate_fee(tx).ok(),
                rate: wallet.calculate_fee_rate(tx).ok(),
            },
            confirmation_time,
        })
    }

    pub async fn get_utxos(&self) -> Vec<LocalOutput> {
        let wallet = self.wallet.read().await;
        wallet.list_unspent().collect()
    }

    async fn internal_full_sync(
        &self,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
        prev_tip: CheckPoint,
        graph: TxGraph<ConfirmationTimeHeightAnchor>,
    ) -> Result<(), Error> {
        let keychain_spks = self.spks().await;

        tracing::info!("Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}");
        let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
        let config: ElectrumConfig = ElectrumConfig::builder()
            .validate_domain(endpoint.validate_tls())
            .timeout(Some(120))
            .retry(3)
            .socks5(proxy)
            .build();
        let client: ElectrumClient =
            ElectrumClient::from_config(&endpoint.as_non_standard_format(), config)?;

        let (
            ElectrumUpdate {
                chain_update,
                relevant_txids,
            },
            keychain_update,
        ) = client.full_scan(prev_tip, keychain_spks, STOP_GAP, BATCH_SIZE)?;
        let missing: Vec<Txid> = relevant_txids.missing_full_txs(&graph);
        let graph_update =
            relevant_txids.into_confirmation_time_tx_graph(&client, None, missing)?;

        let update = Update {
            last_active_indices: keychain_update,
            graph: graph_update,
            chain: Some(chain_update),
        };

        self.apply_update(update).await?;

        Ok(())
    }

    /// Execute a full timechain sync.
    pub async fn full_sync(
        &self,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
        force: bool,
    ) -> Result<(), Error> {
        if !force {
            let last_sync: Timestamp = self.last_sync();
            if last_sync + WALLET_SYNC_INTERVAL > Timestamp::now() {
                return Err(Error::AlreadySynced);
            }

            if self.is_syncing() {
                return Err(Error::AlreadySyncing);
            }
        }

        self.set_syncing(true);

        if force {
            tracing::debug!("Syncing policy {} [full-force]", self.id);
        } else {
            tracing::debug!("Syncing policy {} [full]", self.id);
        }

        // Prepare timechain data
        let prev_tip: CheckPoint = if force {
            CheckPoint::new(BlockId::default())
        } else {
            self.latest_checkpoint().await
        };
        let graph: TxGraph<ConfirmationTimeHeightAnchor> = if force {
            TxGraph::default()
        } else {
            self.graph().await
        };

        // Sync
        self.internal_full_sync(endpoint, proxy, prev_tip, graph)
            .await?;

        // Update sync timestamp and status
        self.update_last_sync();
        self.set_syncing(false);

        if force {
            tracing::info!("Policy {} synced [full-force]", self.id);
        } else {
            tracing::info!("Policy {} synced [full]", self.id);
        }

        Ok(())
    }

    /* pub async fn sync(
        &self,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
    ) -> Result<(), Error> {
        let last_sync: Timestamp = self.last_sync();
        if last_sync + WALLET_SYNC_INTERVAL > Timestamp::now() {
            return Err(Error::AlreadySynced);
        }

        if self.is_syncing() {
            return Err(Error::AlreadySyncing);
        }

        if self.is_chain_empty().await {
            tracing::warn!("Local chain is empty: executing a full sync");
            return self.full_sync(endpoint, proxy, false).await;
        }

        self.set_syncing(true);

        tracing::debug!("Syncing policy {}", self.id);

        let prev_tip: CheckPoint = self.latest_checkpoint().await;
        let chain = self.chain().await;
        let graph: TxGraph<ConfirmationTimeHeightAnchor> = self.graph().await;
        let spk_index = self.spk_index().await;
        let chain_tip: BlockId = chain.tip().block_id();
        let all_spks = spk_index
            .revealed_keychain_spks(&KeychainKind::External)
            .map(|(_, s)| s.to_owned());
        let unconfirmed_txids = graph
            .list_chain_txs(&chain, chain_tip)
            .filter(|canonical_tx| !canonical_tx.chain_position.is_confirmed())
            .map(|canonical_tx| canonical_tx.tx_node.txid);
        let init_outpoints = spk_index.outpoints().iter().cloned();
        let outpoints = graph
            .filter_chain_unspents(&chain, chain_tip, init_outpoints)
            .map(|(_, utxo)| utxo.outpoint);

        tracing::info!("Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}");
        let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
        let config: ElectrumConfig = ElectrumConfig::builder()
            .validate_domain(endpoint.validate_tls())
            .timeout(Some(120))
            .retry(3)
            .socks5(proxy)
            .build();
        let client: ElectrumClient =
            ElectrumClient::from_config(&endpoint.as_non_standard_format(), config)?;

        let ElectrumUpdate {
            chain_update,
            relevant_txids,
        } = client.sync(prev_tip, all_spks, unconfirmed_txids, outpoints, BATCH_SIZE)?;
        let missing: Vec<Txid> = relevant_txids.missing_full_txs(&graph);
        let graph_update =
            relevant_txids.into_confirmation_time_tx_graph(&client, None, missing)?;

        let update = Update {
            last_active_indices: BTreeMap::new(),
            graph: graph_update,
            chain: Some(chain_update),
        };

        self.apply_update(update).await?;
        self.update_last_sync();
        self.set_syncing(false);

        tracing::info!("Policy {} synced", self.id);

        Ok(())
    } */

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn apply_update(&self, update: Update) -> Result<(), Error> {
        let mut wallet = self.wallet.write().await;
        wallet.apply_update(update)?;
        wallet.commit()?;
        Ok(())
    }

    pub async fn estimate_tx_vsize(
        &self,
        address: Address<NetworkUnchecked>,
        amount: Amount,
        utxos: Option<Vec<OutPoint>>,
        frozen_utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Option<usize> {
        let mut wallet = self.wallet.write().await;
        self.policy.estimate_tx_vsize(
            &mut wallet,
            address,
            amount,
            utxos,
            frozen_utxos,
            policy_path,
        )
    }

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
