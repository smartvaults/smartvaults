// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use nostr_ffi::{EventId, Timestamp};
use smartvaults_sdk::core::bdk;
use smartvaults_sdk::core::bdk::chain::ConfirmationTime;
use smartvaults_sdk::core::bitcoin::{self, Address};
use smartvaults_sdk::manager::wallet;
use smartvaults_sdk::types::{self, GetUtxo};
use uniffi::Object;

use crate::error::Result;
use crate::Network;

#[derive(Object)]
pub struct OutPoint {
    inner: bdk::bitcoin::OutPoint,
}

impl From<bdk::bitcoin::OutPoint> for OutPoint {
    fn from(inner: bdk::bitcoin::OutPoint) -> Self {
        Self { inner }
    }
}

impl From<&OutPoint> for bdk::bitcoin::OutPoint {
    fn from(outpoint: &OutPoint) -> Self {
        outpoint.inner
    }
}

#[uniffi::export]
impl OutPoint {
    pub fn txid(&self) -> String {
        self.inner.txid.to_string()
    }

    pub fn vout(&self) -> u32 {
        self.inner.vout
    }
}

#[derive(Object)]
pub struct Utxo {
    inner: GetUtxo,
}

impl From<GetUtxo> for Utxo {
    fn from(inner: GetUtxo) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl Utxo {
    pub fn outpoint(&self) -> Arc<OutPoint> {
        Arc::new(self.inner.utxo.outpoint.into())
    }

    pub fn value(&self) -> u64 {
        self.inner.utxo.txout.value
    }

    pub fn is_spent(&self) -> bool {
        self.inner.utxo.is_spent
    }

    pub fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }
}

#[derive(Object)]
pub struct BlockTime {
    height: u32,
    timestamp: u64,
}

#[uniffi::export]
impl BlockTime {
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn timestamp(&self) -> Timestamp {
        Timestamp::from_secs(self.timestamp)
    }
}

#[derive(Object)]
pub struct TxIn {
    inner: bitcoin::TxIn,
}

impl From<bitcoin::TxIn> for TxIn {
    fn from(inner: bitcoin::TxIn) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl TxIn {
    pub fn previous_output(&self) -> Arc<OutPoint> {
        Arc::new(self.inner.previous_output.into())
    }
}

#[derive(Object)]
pub struct TxOut {
    inner: bitcoin::TxOut,
}

impl From<bitcoin::TxOut> for TxOut {
    fn from(inner: bitcoin::TxOut) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl TxOut {
    pub fn value(&self) -> u64 {
        self.inner.value
    }

    pub fn address(&self, network: Network) -> Result<String> {
        Ok(Address::from_script(&self.inner.script_pubkey, network.into())?.to_string())
    }
}

#[derive(Object)]
pub struct Transaction {
    inner: bitcoin::Transaction,
}

impl From<bitcoin::Transaction> for Transaction {
    fn from(inner: bitcoin::Transaction) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl Transaction {
    pub fn txid(&self) -> String {
        self.inner.txid().to_string()
    }

    pub fn weight(&self) -> u64 {
        self.inner.weight().to_wu()
    }

    pub fn size(&self) -> u64 {
        self.inner.size() as u64
    }

    pub fn vsize(&self) -> u64 {
        self.inner.vsize() as u64
    }

    pub fn is_explicitly_rbf(&self) -> bool {
        self.inner.is_explicitly_rbf()
    }

    pub fn is_lock_time_enabled(&self) -> bool {
        self.inner.is_lock_time_enabled()
    }

    pub fn version(&self) -> i32 {
        self.inner.version
    }

    pub fn lock_time(&self) -> u32 {
        self.inner.lock_time.to_consensus_u32()
    }

    pub fn inputs(&self) -> Vec<Arc<TxIn>> {
        self.inner
            .input
            .clone()
            .into_iter()
            .map(|i| Arc::new(i.into()))
            .collect()
    }

    pub fn outputs(&self) -> Vec<Arc<TxOut>> {
        self.inner
            .output
            .clone()
            .into_iter()
            .map(|i| Arc::new(i.into()))
            .collect()
    }
}

#[derive(Object)]
pub struct TransactionDetails {
    inner: wallet::TransactionDetails,
}

impl From<wallet::TransactionDetails> for TransactionDetails {
    fn from(inner: wallet::TransactionDetails) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl TransactionDetails {
    pub fn txid(&self) -> String {
        self.inner.txid().to_string()
    }

    pub fn received(&self) -> u64 {
        self.inner.received
    }

    pub fn sent(&self) -> u64 {
        self.inner.sent
    }

    pub fn total(&self) -> i64 {
        self.inner.total()
    }

    pub fn fee(&self) -> Option<u64> {
        self.inner.fee
    }

    pub fn confirmation_time(&self) -> Option<Arc<BlockTime>> {
        match self.inner.confirmation_time {
            ConfirmationTime::Confirmed { height, time } => Some(Arc::new(BlockTime {
                height,
                timestamp: time,
            })),
            ConfirmationTime::Unconfirmed { .. } => None,
        }
    }

    pub fn transaction(&self) -> Arc<Transaction> {
        Arc::new(self.inner.transaction.clone().into())
    }
}

#[derive(Object)]
pub struct GetTransaction {
    inner: types::GetTransaction,
}

impl From<types::GetTransaction> for GetTransaction {
    fn from(inner: types::GetTransaction) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl GetTransaction {
    pub fn policy_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.policy_id.into())
    }

    pub fn tx(&self) -> Arc<TransactionDetails> {
        Arc::new(self.inner.tx.clone().into())
    }

    pub fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }

    pub fn block_explorer(&self) -> Option<String> {
        self.inner.block_explorer.clone()
    }
}
