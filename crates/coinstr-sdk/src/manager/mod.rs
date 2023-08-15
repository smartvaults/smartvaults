// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::sync::Arc;

use coinstr_core::bdk::wallet::{AddressIndex, AddressInfo, Balance, NewError};
use coinstr_core::bdk::{FeeRate, LocalUtxo, TransactionDetails, Wallet};
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Address, Network, OutPoint, Script, Txid};
use coinstr_core::{Amount, Policy, Proposal};
use nostr_sdk::hashes::sha256::Hash as Sha256Hash;
use nostr_sdk::hashes::Hash;
use nostr_sdk::EventId;
use parking_lot::RwLock;
use thiserror::Error;

pub mod wallet;

pub use self::wallet::{CoinstrWallet, CoinstrWalletStorage, Error as WalletError, StorageError};
use crate::db::Store;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BdkStore(#[from] NewError<StorageError>),
    #[error(transparent)]
    Wallet(#[from] WalletError),
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

    pub fn load_policy(&self, policy_id: EventId, policy: Policy) -> Result<(), Error> {
        let mut wallets = self.wallets.write();
        if let Entry::Vacant(e) = wallets.entry(policy_id) {
            let descriptor_hash = Sha256Hash::hash(policy.descriptor.to_string().as_bytes());
            let db = CoinstrWalletStorage::new(descriptor_hash, self.db.clone());
            let wallet = Wallet::new(&policy.descriptor.to_string(), None, db, self.network)?;
            let wallet = CoinstrWallet::new(policy, wallet);
            e.insert(wallet);
            tracing::info!("Loaded policy {policy_id}");
            Ok(())
        } else {
            Err(Error::AlreadyLoaded(policy_id))
        }
    }

    pub fn unload_policy(&self, policy_id: EventId) -> Result<(), Error> {
        let mut wallets = self.wallets.write();
        match wallets.remove(&policy_id) {
            Some(_) => Ok(()),
            None => Err(Error::NotLoaded(policy_id)),
        }
    }

    pub fn wallet(&self, policy_id: EventId) -> Result<CoinstrWallet, Error> {
        let wallets = self.wallets.read();
        Ok(wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .clone())
    }

    pub fn get_balance(&self, policy_id: EventId) -> Result<Balance, Error> {
        Ok(self.wallet(policy_id)?.get_balance())
    }

    pub fn get_address(
        &self,
        policy_id: EventId,
        index: AddressIndex,
    ) -> Result<AddressInfo, Error> {
        Ok(self.wallet(policy_id)?.get_address(index))
    }

    pub fn get_addresses(&self, policy_id: EventId) -> Result<Vec<Address>, Error> {
        Ok(self.wallet(policy_id)?.get_addresses()?)
    }

    pub fn get_addresses_balances(
        &self,
        policy_id: EventId,
    ) -> Result<HashMap<Script, u64>, Error> {
        Ok(self.wallet(policy_id)?.get_addresses_balances())
    }

    pub fn get_txs(&self, policy_id: EventId) -> Result<Vec<TransactionDetails>, Error> {
        Ok(self.wallet(policy_id)?.get_txs())
    }

    pub fn get_tx(&self, policy_id: EventId, txid: Txid) -> Result<TransactionDetails, Error> {
        Ok(self.wallet(policy_id)?.get_tx(txid)?)
    }

    pub fn get_utxos(&self, policy_id: EventId) -> Result<Vec<LocalUtxo>, Error> {
        Ok(self.wallet(policy_id)?.get_utxos())
    }

    pub fn sync<S>(
        &self,
        policy_id: EventId,
        endpoint: S,
        proxy: Option<SocketAddr>,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        Ok(self.wallet(policy_id)?.sync(endpoint, proxy)?)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spend<S>(
        &self,
        policy_id: EventId,
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
        Ok(self.wallet(policy_id)?.spend(
            address,
            amount,
            description,
            fee_rate,
            utxos,
            policy_path,
        )?)
    }

    pub fn proof_of_reserve<S>(&self, policy_id: EventId, message: S) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        Ok(self.wallet(policy_id)?.proof_of_reserve(message)?)
    }

    pub fn verify_proof<S>(
        &self,
        policy_id: EventId,
        psbt: &PartiallySignedTransaction,
        message: S,
    ) -> Result<u64, Error>
    where
        S: Into<String>,
    {
        Ok(self.wallet(policy_id)?.verify_proof(psbt, message)?)
    }
}
