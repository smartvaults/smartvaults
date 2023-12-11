// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use async_utility::futures_util::stream;
use uniffi::Object;

#[derive(Object)]
pub struct AbortHandle {
    inner: stream::AbortHandle,
}

impl From<stream::AbortHandle> for AbortHandle {
    fn from(inner: stream::AbortHandle) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl AbortHandle {
    /// Abort thread
    pub fn abort(&self) {
        if self.is_aborted() {
            tracing::warn!("Thread already aborted");
        } else {
            self.inner.abort();
            tracing::info!("Thread aborted!");
        }
    }

    /// Check if thread is aborted
    pub fn is_aborted(&self) -> bool {
        self.inner.is_aborted()
    }
}

impl Drop for AbortHandle {
    fn drop(&mut self) {
        tracing::info!("Dropping AbortHandle...");
        self.abort();
    }
}
