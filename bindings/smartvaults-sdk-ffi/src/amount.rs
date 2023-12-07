// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::sync::Arc;

use smartvaults_sdk::core;
use uniffi::Object;

#[derive(Object)]
pub struct Amount {
    inner: core::Amount,
}

impl Deref for Amount {
    type Target = core::Amount;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[uniffi::export]
impl Amount {
    #[uniffi::constructor]
    pub fn custom(amount: u64) -> Arc<Self> {
        Arc::new(Self {
            inner: core::Amount::Custom(amount),
        })
    }

    #[uniffi::constructor]
    pub fn max() -> Arc<Self> {
        Arc::new(Self {
            inner: core::Amount::Max,
        })
    }
}
