// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use async_utility::thread;
use bdk_electrum::electrum_client::{
    self, Client as ElectrumClient, Config as ElectrumConfig, ElectrumApi, HeaderNotification,
    Socks5Config,
};
use nostr_sdk::hashes::sha256::Hash as Sha256Hash;
use nostr_sdk::hashes::Hash;
use nostr_sdk::Timestamp;
use smartvaults_core::bdk::chain::ConfirmationTime;
use smartvaults_core::bdk::wallet::{AddressIndex, AddressInfo, Balance, NewOrLoadError};
use smartvaults_core::bdk::{FeeRate, LocalOutput, Wallet};
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::{Address, Network, OutPoint, ScriptBuf, Transaction, Txid};
use smartvaults_core::{Destination, Policy, Priority, ProofOfReserveProposal, SpendingProposal};
use smartvaults_protocol::v2::VaultIdentifier;
use smartvaults_sdk_sqlite::Store;
use thiserror::Error;
use tokio::sync::broadcast::Sender;
use tokio::sync::RwLock;

pub mod wallet;

pub use self::wallet::{
    Error as WalletError, SmartVaultsWallet, SmartVaultsWalletStorage, StorageError,
    TransactionDetails,
};
use crate::config::ElectrumEndpoint;
use crate::constants::{BLOCK_HEIGHT_SYNC_INTERVAL, MEMPOOL_TX_FEES_SYNC_INTERVAL};
use crate::Message;

const TARGET_BLOCKS: [Priority; 3] = [Priority::High, Priority::Medium, Priority::Low];

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Thread(#[from] async_utility::thread::Error),
    #[error(transparent)]
    BdkStore(#[from] NewOrLoadError<StorageError, StorageError>),
    #[error(transparent)]
    Electrum(#[from] electrum_client::Error),
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("vault {0} already loaded")]
    AlreadyLoaded(VaultIdentifier),
    #[error("vault {0} not loaded")]
    NotLoaded(VaultIdentifier),
}

#[derive(Debug, Clone, Default)]
pub struct BlockHeight {
    height: Arc<AtomicU32>,
    last_sync: Arc<RwLock<Option<Timestamp>>>,
}

impl BlockHeight {
    pub fn block_height(&self) -> u32 {
        self.height.load(Ordering::SeqCst)
    }

    pub fn set_block_height(&self, block_height: u32) {
        let _ = self
            .height
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(block_height));
    }

    pub async fn is_synced(&self) -> bool {
        let last_sync = self.last_sync.read().await;
        let last_sync: Timestamp = last_sync.unwrap_or_else(|| Timestamp::from(0));
        last_sync.add(BLOCK_HEIGHT_SYNC_INTERVAL) > Timestamp::now()
    }

    pub async fn just_synced(&self) {
        let mut last_sync = self.last_sync.write().await;
        *last_sync = Some(Timestamp::now());
    }
}

#[derive(Debug, Clone, Default)]
pub struct EstimatedMempoolFees {
    fees: Arc<RwLock<BTreeMap<Priority, FeeRate>>>,
    last_sync: Arc<RwLock<Option<Timestamp>>>,
}

impl EstimatedMempoolFees {
    pub async fn get(&self) -> BTreeMap<Priority, FeeRate> {
        self.fees.read().await.clone()
    }

    pub async fn set_fees(&self, fees: BTreeMap<Priority, FeeRate>) {
        let mut f = self.fees.write().await;
        *f = fees;
    }

    pub async fn is_synced(&self) -> bool {
        let last_sync = self.last_sync.read().await;
        let last_sync: Timestamp = last_sync.unwrap_or_else(|| Timestamp::from(0));
        last_sync.add(MEMPOOL_TX_FEES_SYNC_INTERVAL) > Timestamp::now()
    }

    pub async fn just_synced(&self) {
        let mut last_sync = self.last_sync.write().await;
        *last_sync = Some(Timestamp::now());
    }
}

#[derive(Debug, Clone)]
pub struct Manager {
    db: Store,
    network: Network,
    wallets: Arc<RwLock<HashMap<VaultIdentifier, SmartVaultsWallet>>>,
    block_height: BlockHeight,
    mempool_fees: EstimatedMempoolFees,
}

impl Manager {
    pub fn new(db: Store, network: Network) -> Self {
        Self {
            db,
            network,
            wallets: Arc::new(RwLock::new(HashMap::new())),
            block_height: BlockHeight::default(),
            mempool_fees: EstimatedMempoolFees::default(),
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn load_policy(
        &self,
        vault_id: VaultIdentifier,
        policy: Policy,
    ) -> Result<(), Error> {
        let this = self.clone();
        let mut wallets = self.wallets.write().await;
        if let Entry::Vacant(e) = wallets.entry(vault_id) {
            let wallet: SmartVaultsWallet = tokio::task::spawn_blocking(move || {
                let desc: String = policy.as_descriptor().to_string();
                let descriptor_hash = Sha256Hash::hash(desc.as_bytes());
                let db: SmartVaultsWalletStorage =
                    SmartVaultsWalletStorage::new(descriptor_hash, this.db.clone());
                let wallet: Wallet<SmartVaultsWalletStorage> =
                    Wallet::new_or_load(&desc, None, db, this.network)?;
                Ok::<SmartVaultsWallet, Error>(SmartVaultsWallet::new(policy_id, policy, wallet))
            })
            .await??;
            e.insert(wallet);
            tracing::info!("Loaded policy {vault_id}");
            Ok(())
        } else {
            Err(Error::AlreadyLoaded(vault_id))
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn unload_policies(&self) {
        let mut wallets = self.wallets.write().await;
        wallets.clear();
        tracing::info!("All policies unloaded.")
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn unload_policy(&self, vault_id: VaultIdentifier) -> Result<(), Error> {
        let mut wallets = self.wallets.write().await;
        match wallets.remove(&vault_id) {
            Some(_) => Ok(()),
            None => Err(Error::NotLoaded(vault_id)),
        }
    }

    pub fn block_height(&self) -> u32 {
        self.block_height.block_height()
    }

    pub async fn sync_block_height(
        &self,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
    ) -> Result<(), Error> {
        if !self.block_height.is_synced().await {
            tracing::info!("Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}");
            let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
            let config = ElectrumConfig::builder()
                .validate_domain(endpoint.validate_tls())
                .socks5(proxy)
                .build();
            let client = ElectrumClient::from_config(&endpoint.as_non_standard_format(), config)?;

            let HeaderNotification { height, .. } = client.block_headers_subscribe()?;
            let height: u32 = height as u32;

            if self.block_height() != height {
                self.block_height.set_block_height(height);
                self.block_height.just_synced().await;

                tracing::info!("Block height synced")
            }
        }

        Ok(())
    }

    pub async fn sync_mempool_fees(
        &self,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
    ) -> Result<Option<BTreeMap<Priority, FeeRate>>, Error> {
        if !self.mempool_fees.is_synced().await {
            tracing::info!("Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}");
            let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
            let config = ElectrumConfig::builder()
                .validate_domain(endpoint.validate_tls())
                .socks5(proxy)
                .build();
            let client = ElectrumClient::from_config(&endpoint.as_non_standard_format(), config)?;

            let fees: Vec<f64> = client
                .batch_estimate_fee(TARGET_BLOCKS.iter().map(|p| p.target_blocks() as usize))?;
            if TARGET_BLOCKS.len() == fees.len() {
                let mut estimated_fees = BTreeMap::new();
                for (priority, btc_per_kvb) in TARGET_BLOCKS.into_iter().zip(fees) {
                    let rate = FeeRate::from_btc_per_kvb(btc_per_kvb as f32);
                    estimated_fees.insert(priority, rate);
                }

                // Save
                self.mempool_fees.set_fees(estimated_fees.clone()).await;
                self.mempool_fees.just_synced().await;
                tracing::info!("Mempool fees synced");

                return Ok(Some(estimated_fees));
            }
        }

        Ok(None)
    }

    pub async fn wallet(&self, vault_id: &VaultIdentifier) -> Result<SmartVaultsWallet, Error> {
        let wallets = self.wallets.read().await;
        Ok(wallets
            .get(vault_id)
            .ok_or(Error::NotLoaded(*vault_id))?
            .clone())
    }

    pub async fn insert_tx(
        &self,
        vault_id: &VaultIdentifier,
        tx: Transaction,
        position: ConfirmationTime,
    ) -> Result<bool, Error> {
        Ok(self.wallet(vault_id).await?.insert_tx(tx, position).await?)
    }

    pub async fn last_sync(&self, policy_id: EventId) -> Result<Timestamp, Error> {
        Ok(self.wallet(policy_id).await?.last_sync())
    }

    pub async fn get_balance(&self, vault_id: &VaultIdentifier) -> Result<Balance, Error> {
        Ok(self.wallet(vault_id).await?.get_balance().await)
    }

    pub async fn get_address(
        &self,
        vault_id: &VaultIdentifier,
        index: AddressIndex,
    ) -> Result<AddressInfo, Error> {
        Ok(self.wallet(vault_id).await?.get_address(index).await?)
    }

    pub async fn get_addresses(
        &self,
        vault_id: &VaultIdentifier,
    ) -> Result<Vec<Address<NetworkUnchecked>>, Error> {
        Ok(self.wallet(vault_id).await?.get_addresses().await?)
    }

    pub async fn get_addresses_balances(
        &self,
        vault_id: &VaultIdentifier,
    ) -> Result<HashMap<ScriptBuf, u64>, Error> {
        Ok(self.wallet(vault_id).await?.get_addresses_balances().await)
    }

    pub async fn get_txs(
        &self,
        vault_id: &VaultIdentifier,
    ) -> Result<BTreeSet<TransactionDetails>, Error> {
        Ok(self.wallet(vault_id).await?.txs().await)
    }

    pub async fn get_tx(
        &self,
        vault_id: &VaultIdentifier,
        txid: Txid,
    ) -> Result<TransactionDetails, Error> {
        Ok(self.wallet(vault_id).await?.get_tx(txid).await?)
    }

    pub async fn get_utxos(&self, vault_id: &VaultIdentifier) -> Result<Vec<LocalOutput>, Error> {
        Ok(self.wallet(vault_id).await?.get_utxos().await)
    }

    /// Sync all policies with the timechain
    pub async fn sync_all(
        &self,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
        sync_channel: Option<Sender<Message>>,
    ) -> Result<(), Error> {
        let wallets = self.wallets.read().await;
        for (id, wallet) in wallets.clone().into_iter() {
            let endpoint = endpoint.clone();
            let sync_channel = sync_channel.clone();
            thread::spawn(async move {
                match wallet.full_sync(endpoint, proxy, false).await {
                    Ok(_) => {
                        if let Some(sync_channel) = sync_channel {
                            let _ = sync_channel.send(Message::WalletSyncCompleted(id));
                        }
                    }
                    Err(WalletError::AlreadySynced) => {}
                    Err(WalletError::AlreadySyncing) => {
                        tracing::warn!("Policy {id} is already syncing");
                    }
                    Err(e) => tracing::error!("Impossible to sync policy {id}: {e}"),
                }
            })?;
        }
        Ok(())
    }

    /* /// Execute a timechain sync
    ///
    /// If the local chain is empty, execute a full sync.
    pub async fn sync(
        &self,
        vault_id: &VaultIdentifier,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
    ) -> Result<(), Error> {
        Ok(self.wallet(vault_id).await?.sync(endpoint, proxy).await?)
    } */

    /// Full sync all policies with the timechain
    pub async fn full_sync_all(
        &self,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
        force: bool,
        sync_channel: Option<Sender<Message>>,
    ) -> Result<(), Error> {
        let wallets = self.wallets.read().await;
        for (id, wallet) in wallets.clone().into_iter() {
            let endpoint = endpoint.clone();
            let sync_channel = sync_channel.clone();
            thread::spawn(async move {
                match wallet.full_sync(endpoint, proxy, force).await {
                    Ok(_) => {
                        if let Some(sync_channel) = sync_channel {
                            let _ = sync_channel.send(Message::WalletSyncCompleted(id));
                        }
                    }
                    Err(WalletError::AlreadySynced) => {}
                    Err(WalletError::AlreadySyncing) => {
                        tracing::warn!("Policy {id} is already syncing");
                    }
                    Err(e) => tracing::error!("Impossible to sync policy {id}: {e}"),
                }
            })?;
        }
        Ok(())
    }

    /// Execute a **full** timechain sync.
    pub async fn full_sync(
        &self,
        vault_id: &VaultIdentifier,
        endpoint: ElectrumEndpoint,
        proxy: Option<SocketAddr>,
        force: bool,
    ) -> Result<(), Error> {
        Ok(self
            .wallet(vault_id)
            .await?
            .full_sync(endpoint, proxy, force)
            .await?)
    }

    pub async fn spend(
        &self,
        vault_id: &VaultIdentifier,
        destination: &Destination,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        frozen_utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
    ) -> Result<SpendingProposal, Error> {
        Ok(self
            .wallet(vault_id)
            .await?
            .spend(destination, fee_rate, utxos, frozen_utxos, policy_path)
            .await?)
    }

    pub async fn proof_of_reserve<S>(
        &self,
        vault_id: &VaultIdentifier,
        message: S,
    ) -> Result<ProofOfReserveProposal, Error>
    where
        S: Into<String>,
    {
        Ok(self
            .wallet(vault_id)
            .await?
            .proof_of_reserve(message)
            .await?)
    }

    pub async fn verify_proof<S>(
        &self,
        vault_id: &VaultIdentifier,
        psbt: &PartiallySignedTransaction,
        message: S,
    ) -> Result<u64, Error>
    where
        S: Into<String>,
    {
        Ok(self
            .wallet(vault_id)
            .await?
            .verify_proof(psbt, message)
            .await?)
    }
}
