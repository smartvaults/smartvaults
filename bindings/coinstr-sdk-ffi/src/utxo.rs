// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::core::bdk::{self, LocalUtxo};

pub struct OutPoint {
    inner: bdk::bitcoin::OutPoint,
}

impl From<bdk::bitcoin::OutPoint> for OutPoint {
    fn from(inner: bdk::bitcoin::OutPoint) -> Self {
        Self { inner }
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
    inner: LocalUtxo,
}

impl From<LocalUtxo> for Utxo {
    fn from(inner: LocalUtxo) -> Self {
        Self { inner }
    }
}

impl Utxo {
    pub fn outpoint(&self) -> Arc<OutPoint> {
        Arc::new(self.inner.outpoint.into())
    }

    pub fn value(&self) -> u64 {
        self.inner.txout.value
    }

    pub fn is_spent(&self) -> bool {
        self.inner.is_spent
    }
}
