// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::path::Path;

use coinstr_core::bdk::wallet::{AddressIndex, AddressInfo, Balance, NewError};
use coinstr_core::bdk::{FeeRate, LocalUtxo, TransactionDetails, Wallet};
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Address, Network, OutPoint, Script, Txid};
use coinstr_core::{Amount, Policy, Proposal};
use dashmap::DashMap;
use nostr::EventId;
use sled::Db;
use thiserror::Error;

use crate::storage::CoinstrWalletStorage;
use crate::storage::Error as StorageError;
use crate::wallet::CoinstrWallet;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Sled(#[from] sled::Error),
    #[error(transparent)]
    BdkStore(#[from] NewError<StorageError>),
    #[error(transparent)]
    Wallet(#[from] crate::wallet::Error),
    #[error("policy {0} already loaded")]
    AlreadyLoaded(EventId),
    #[error("policy {0} not loaded")]
    NotLoaded(EventId),
}

#[derive(Debug, Clone)]
pub struct Manager {
    db: Db,
    network: Network,
    wallets: DashMap<EventId, CoinstrWallet>,
}

impl Manager {
    pub fn new<P>(path: P, network: Network) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            db: sled::open(path)?,
            network,
            wallets: DashMap::new(),
        })
    }

    pub fn load_policy(&self, policy_id: EventId, policy: Policy) -> Result<(), Error> {
        if !self.wallets.contains_key(&policy_id) {
            let tree = self.db.open_tree(policy_id)?;
            let db = CoinstrWalletStorage::new(tree);
            let wallet = Wallet::new(&policy.descriptor.to_string(), None, db, self.network)?;
            let wallet = CoinstrWallet::new(policy, wallet);
            self.wallets.insert(policy_id, wallet);
            Ok(())
        } else {
            Err(Error::AlreadyLoaded(policy_id))
        }
    }

    pub fn unload_policy(&self, policy_id: EventId) -> Result<(), Error> {
        match self.wallets.remove(&policy_id) {
            Some(_) => Ok(()),
            None => Err(Error::NotLoaded(policy_id)),
        }
    }

    pub fn wallet(&self, policy_id: EventId) -> Result<CoinstrWallet, Error> {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .clone())
    }

    pub fn get_balance(&self, policy_id: EventId) -> Result<Balance, Error> {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .get_balance())
    }

    pub fn get_address(
        &self,
        policy_id: EventId,
        index: AddressIndex,
    ) -> Result<AddressInfo, Error> {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .get_address(index))
    }

    pub fn get_addresses(&self, policy_id: EventId) -> Result<Vec<Address>, Error> {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .get_addresses()?)
    }

    pub fn get_addresses_balances(
        &self,
        policy_id: EventId,
    ) -> Result<HashMap<Script, u64>, Error> {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .get_addresses_balances())
    }

    pub fn get_txs(&self, policy_id: EventId) -> Result<Vec<TransactionDetails>, Error> {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .get_txs())
    }

    pub fn get_tx(&self, policy_id: EventId, txid: Txid) -> Result<TransactionDetails, Error> {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .get_tx(txid)?)
    }

    pub fn get_utxos(&self, policy_id: EventId) -> Result<Vec<LocalUtxo>, Error> {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .get_utxos())
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
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .sync(endpoint, proxy)?)
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
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .spend(address, amount, description, fee_rate, utxos, policy_path)?)
    }

    pub fn proof_of_reserve<S>(&self, policy_id: EventId, message: S) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .proof_of_reserve(message)?)
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
        Ok(self
            .wallets
            .get(&policy_id)
            .ok_or(Error::NotLoaded(policy_id))?
            .verify_proof(psbt, message)?)
    }
}
