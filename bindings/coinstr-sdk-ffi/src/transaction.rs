// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::core::bdk;
use coinstr_sdk::core::bdk::chain::ConfirmationTime;
use coinstr_sdk::core::bitcoin::{self, Address};
use coinstr_sdk::db::model::{self, GetUtxo};
use nostr_sdk_ffi::Timestamp;

use crate::error::Result;
use crate::Network;

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

impl OutPoint {
    pub fn txid(&self) -> String {
        self.inner.txid.to_string()
    }

    pub fn vout(&self) -> u32 {
        self.inner.vout
    }
}

pub struct Utxo {
    inner: GetUtxo,
}

impl From<GetUtxo> for Utxo {
    fn from(inner: GetUtxo) -> Self {
        Self { inner }
    }
}

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

pub struct BlockTime {
    height: u32,
    timestamp: u64,
}

impl BlockTime {
    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn timestamp(&self) -> Arc<Timestamp> {
        Arc::new(Timestamp::from_secs(self.timestamp))
    }
}

pub struct TxIn {
    inner: bitcoin::TxIn,
}

impl From<bitcoin::TxIn> for TxIn {
    fn from(inner: bitcoin::TxIn) -> Self {
        Self { inner }
    }
}

impl TxIn {
    pub fn previous_output(&self) -> Arc<OutPoint> {
        Arc::new(self.inner.previous_output.into())
    }
}

pub struct TxOut {
    inner: bitcoin::TxOut,
}

impl From<bitcoin::TxOut> for TxOut {
    fn from(inner: bitcoin::TxOut) -> Self {
        Self { inner }
    }
}

impl TxOut {
    pub fn value(&self) -> u64 {
        self.inner.value
    }

    pub fn address(&self, network: Network) -> Result<String> {
        Ok(Address::from_script(&self.inner.script_pubkey, network.into())?.to_string())
    }
}

pub struct Transaction {
    inner: bitcoin::Transaction,
}

impl From<bitcoin::Transaction> for Transaction {
    fn from(inner: bitcoin::Transaction) -> Self {
        Self { inner }
    }
}

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

pub struct TransactionDetails {
    inner: bdk::TransactionDetails,
}

impl From<bdk::TransactionDetails> for TransactionDetails {
    fn from(inner: bdk::TransactionDetails) -> Self {
        Self { inner }
    }
}

impl TransactionDetails {
    pub fn txid(&self) -> String {
        self.inner.txid.to_string()
    }

    pub fn received(&self) -> u64 {
        self.inner.received
    }

    pub fn sent(&self) -> u64 {
        self.inner.sent
    }

    pub fn total(&self) -> i64 {
        let received = self.inner.received as i64;
        let sent = self.inner.sent as i64;
        received.saturating_sub(sent)
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

    pub fn transaction(&self) -> Option<Arc<Transaction>> {
        self.inner.transaction.clone().map(|tx| Arc::new(tx.into()))
    }
}

pub struct GetTransaction {
    inner: model::GetTransaction,
}

impl From<model::GetTransaction> for GetTransaction {
    fn from(inner: model::GetTransaction) -> Self {
        Self { inner }
    }
}

impl GetTransaction {
    pub fn tx(&self) -> Arc<TransactionDetails> {
        Arc::new(self.inner.tx.clone().into())
    }

    pub fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }
}
