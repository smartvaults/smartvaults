// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::nostr::{self, block_on, RelayStatus};

pub struct Relay {
    inner: nostr::Relay,
}

impl From<nostr::Relay> for Relay {
    fn from(inner: nostr::Relay) -> Self {
        Self { inner }
    }
}

impl Relay {
    pub fn url(&self) -> String {
        self.inner.url().to_string()
    }

    pub fn status(&self) -> RelayStatus {
        self.inner.status_blocking()
    }

    pub fn is_connected(&self) -> bool {
        block_on(async move { self.inner.is_connected().await })
    }
}
