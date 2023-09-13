// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::core::bdk;

pub struct Balance {
    inner: bdk::wallet::Balance,
}

impl From<bdk::wallet::Balance> for Balance {
    fn from(inner: bdk::wallet::Balance) -> Self {
        Self { inner }
    }
}

impl Balance {
    /// Get sum of trusted_pending and confirmed coins
    pub fn get_spendable(&self) -> u64 {
        self.inner.trusted_spendable()
    }

    /// Get the whole balance visible to the wallet
    pub fn get_total(&self) -> u64 {
        self.inner.total()
    }
}
