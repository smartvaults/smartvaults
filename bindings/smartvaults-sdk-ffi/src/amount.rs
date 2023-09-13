// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::core;

pub struct Amount {
    inner: core::Amount,
}

impl Amount {
    pub fn custom(amount: u64) -> Self {
        Self {
            inner: core::Amount::Custom(amount),
        }
    }

    pub fn max() -> Self {
        Self {
            inner: core::Amount::Max,
        }
    }

    pub(crate) fn inner(&self) -> core::Amount {
        self.inner
    }
}
