// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use async_utility::futures_util::stream;

pub struct AbortHandle {
    inner: stream::AbortHandle,
}

impl From<stream::AbortHandle> for AbortHandle {
    fn from(inner: stream::AbortHandle) -> Self {
        Self { inner }
    }
}

impl AbortHandle {
    pub fn abort(&self) {
        self.inner.abort()
    }

    pub fn is_aborted(&self) -> bool {
        self.inner.is_aborted()
    }
}

impl Drop for AbortHandle {
    fn drop(&mut self) {
        if self.is_aborted() {
            tracing::warn!("AbortHanlde already aborted");
        } else {
            self.abort();
            tracing::info!("AbortHandle dropped");
        }
    }
}
