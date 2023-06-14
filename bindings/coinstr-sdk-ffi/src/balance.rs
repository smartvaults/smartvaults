// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bdk;

pub struct Balance {
    inner: bdk::Balance,
}

impl From<bdk::Balance> for Balance {
    fn from(inner: bdk::Balance) -> Self {
        Self { inner }
    }
}

impl Balance {
    /// Get sum of trusted_pending and confirmed coins
    pub fn get_spendable(&self) -> u64 {
        self.inner.get_spendable()
    }

    /// Get the whole balance visible to the wallet
    pub fn get_total(&self) -> u64 {
        self.inner.get_total()
    }
}
