// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::core::bdk;

pub struct BlockTime {
    inner: bdk::BlockTime,
}

impl From<bdk::BlockTime> for BlockTime {
    fn from(inner: bdk::BlockTime) -> Self {
        Self { inner }
    }
}

impl BlockTime {
    pub fn height(&self) -> u32 {
        self.inner.height
    }

    pub fn timestamp(&self) -> u64 {
        self.inner.timestamp
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

    pub fn fee(&self) -> Option<u64> {
        self.inner.fee
    }

    pub fn confirmation_time(&self) -> Option<Arc<BlockTime>> {
        self.inner
            .confirmation_time
            .clone()
            .map(|b| Arc::new(b.into()))
    }
}