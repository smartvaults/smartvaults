// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;

use coinstr_core::bdk::wallet::{AddressIndex, AddressInfo, Balance, NewError};
use coinstr_core::bdk::{FeeRate, LocalUtxo, Wallet};
use coinstr_core::bitcoin::address::NetworkUnchecked;
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Address, Network, OutPoint, ScriptBuf, Txid};
use coinstr_core::{Amount, Policy, Proposal};
use nostr_sdk::hashes::sha256::Hash as Sha256Hash;
use nostr_sdk::hashes::Hash;
use nostr_sdk::EventId;
use thiserror::Error;
use tokio::sync::RwLock;

pub mod wallet;

pub use self::wallet::{
    CoinstrWallet, CoinstrWalletStorage, Error as WalletError, StorageError, TransactionDetails,
};
use crate::db::Store;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BdkStore(#[from] NewError<StorageError>),
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    #[error("policy {0} already loaded")]
    AlreadyLoaded(EventId),
    #[error("policy {0} not loaded")]
    NotLoaded(EventId),
}

#[derive(Debug, Clone)]
pub struct Manager {
    db: Store,
    network: Network,
    wallets: Arc<RwLock<HashMap<EventId, CoinstrWallet>>>,
}

impl Manager {
    pub fn new(db: Store, network: Network) -> Self {
        Self {
            db,
            network,
            wallets: Arc::new(RwLock::new(HashMap::new())),
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

    pub async fn wallet(&self, policy_id: EventId) -> Result<CoinstrWallet, Error> {
        let wallets = self.wallets.read().await;
        Ok(wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .clone())
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
