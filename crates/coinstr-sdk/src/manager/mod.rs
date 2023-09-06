// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::ops::Add;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use bdk_electrum::bdk_chain::ConfirmationTime;
use bdk_electrum::electrum_client::ElectrumApi;
use bdk_electrum::electrum_client::{
    self, Client as ElectrumClient, Config as ElectrumConfig, HeaderNotification, Socks5Config,
};
use coinstr_core::bdk::wallet::{AddressIndex, AddressInfo, Balance, NewError};
use coinstr_core::bdk::{FeeRate, LocalUtxo, Wallet};
use coinstr_core::bitcoin::address::NetworkUnchecked;
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Address, Network, OutPoint, ScriptBuf, Transaction, Txid};
use coinstr_core::{Amount, Policy, Proposal};
use coinstr_sdk_sqlite::Store;
use nostr_sdk::hashes::sha256::Hash as Sha256Hash;
use nostr_sdk::hashes::Hash;
use nostr_sdk::{EventId, Timestamp};
use thiserror::Error;
use tokio::sync::RwLock;

pub mod wallet;

pub use self::wallet::{
    CoinstrWallet, CoinstrWalletStorage, Error as WalletError, StorageError, TransactionDetails,
};
use crate::constants::BLOCK_HEIGHT_SYNC_INTERVAL;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BdkStore(#[from] NewError<StorageError>),
    #[error(transparent)]
    Electrum(#[from] electrum_client::Error),
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("policy {0} already loaded")]
    AlreadyLoaded(EventId),
    #[error("policy {0} not loaded")]
    NotLoaded(EventId),
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

#[derive(Debug, Clone)]
pub struct Manager {
    db: Store,
    network: Network,
    wallets: Arc<RwLock<HashMap<EventId, CoinstrWallet>>>,
    block_height: BlockHeight,
}

impl Manager {
    pub fn new(db: Store, network: Network) -> Self {
        Self {
            db,
            network,
            wallets: Arc::new(RwLock::new(HashMap::new())),
            block_height: BlockHeight::default(),
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn load_policy(&self, policy_id: EventId, policy: Policy) -> Result<(), Error> {
        let this = self.clone();
        let mut wallets = self.wallets.write().await;
        if let Entry::Vacant(e) = wallets.entry(policy_id) {
            let wallet: CoinstrWallet = tokio::task::spawn_blocking(move || {
                let descriptor_hash = Sha256Hash::hash(policy.descriptor.to_string().as_bytes());
                let db: CoinstrWalletStorage =
                    CoinstrWalletStorage::new(descriptor_hash, this.db.clone());
                let wallet: Wallet<CoinstrWalletStorage> =
                    Wallet::new(&policy.descriptor.to_string(), None, db, this.network)?;
                Ok::<CoinstrWallet, Error>(CoinstrWallet::new(policy, wallet))
            })
            .await??;
            e.insert(wallet);
            tracing::info!("Loaded policy {policy_id}");
            Ok(())
        } else {
            Err(Error::AlreadyLoaded(policy_id))
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn unload_policy(&self, policy_id: EventId) -> Result<(), Error> {
        let mut wallets = self.wallets.write().await;
        match wallets.remove(&policy_id) {
            Some(_) => Ok(()),
            None => Err(Error::NotLoaded(policy_id)),
        }
    }

    pub fn block_height(&self) -> u32 {
        self.block_height.block_height()
    }

    pub async fn sync_block_height<S>(
        &self,
        endpoint: S,
        proxy: Option<SocketAddr>,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        if !self.block_height.is_synced().await {
            let endpoint: String = endpoint.into();

            tracing::info!("Initializing electrum client: endpoint={endpoint}, proxy={proxy:?}");
            let proxy: Option<Socks5Config> = proxy.map(Socks5Config::new);
            let config = ElectrumConfig::builder().socks5(proxy).build();
            let client = ElectrumClient::from_config(&endpoint, config)?;

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

    pub async fn wallet(&self, policy_id: EventId) -> Result<CoinstrWallet, Error> {
        let wallets = self.wallets.read().await;
        Ok(wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .clone())
    }

    pub async fn insert_tx(
        &self,
        policy_id: EventId,
        tx: Transaction,
        position: ConfirmationTime,
    ) -> Result<bool, Error> {
        Ok(self
            .wallet(policy_id)
            .await?
            .insert_tx(tx, position)
            .await?)
    }

    pub async fn get_balance(&self, policy_id: EventId) -> Result<Balance, Error> {
        Ok(self.wallet(policy_id).await?.get_balance().await)
    }

    pub async fn get_address(
        &self,
        policy_id: EventId,
        index: AddressIndex,
    ) -> Result<AddressInfo, Error> {
        Ok(self.wallet(policy_id).await?.get_address(index).await)
    }

    pub async fn get_addresses(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<Address<NetworkUnchecked>>, Error> {
        Ok(self.wallet(policy_id).await?.get_addresses().await?)
    }

    pub async fn get_addresses_balances(
        &self,
        policy_id: EventId,
    ) -> Result<HashMap<ScriptBuf, u64>, Error> {
        Ok(self.wallet(policy_id).await?.get_addresses_balances().await)
    }

    pub async fn get_txs(&self, policy_id: EventId) -> Result<Vec<TransactionDetails>, Error> {
        Ok(self.wallet(policy_id).await?.get_txs().await)
    }

    pub async fn get_tx(
        &self,
        policy_id: EventId,
        txid: Txid,
    ) -> Result<TransactionDetails, Error> {
        Ok(self.wallet(policy_id).await?.get_tx(txid).await?)
    }

    pub async fn get_utxos(&self, policy_id: EventId) -> Result<Vec<LocalUtxo>, Error> {
        Ok(self.wallet(policy_id).await?.get_utxos().await)
    }

    pub async fn sync<S>(
        &self,
        policy_id: EventId,
        endpoint: S,
        proxy: Option<SocketAddr>,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        Ok(self.wallet(policy_id).await?.sync(endpoint, proxy).await?)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn spend<S>(
        &self,
        policy_id: EventId,
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
        Ok(self
            .wallet(policy_id)
            .await?
            .spend(
                address,
                amount,
                description,
                fee_rate,
                utxos,
                frozen_utxos,
                policy_path,
            )
            .await?)
    }

    pub async fn proof_of_reserve<S>(
        &self,
        policy_id: EventId,
        message: S,
    ) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        Ok(self
            .wallet(policy_id)
            .await?
            .proof_of_reserve(message)
            .await?)
    }

    pub async fn verify_proof<S>(
        &self,
        policy_id: EventId,
        psbt: &PartiallySignedTransaction,
        message: S,
    ) -> Result<u64, Error>
    where
        S: Into<String>,
    {
        Ok(self
            .wallet(policy_id)
            .await?
            .verify_proof(psbt, message)
            .await?)
    }
}
