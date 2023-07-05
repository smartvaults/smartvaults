// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::nostr;

#[derive(Clone)]
pub struct Metadata {
    inner: nostr::Metadata,
}

impl From<nostr::Metadata> for Metadata {
    fn from(inner: nostr::Metadata) -> Self {
        Self { inner }
    }
}

impl Metadata {
    pub fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    pub fn display_name(&self) -> Option<String> {
        self.inner.display_name.clone()
    }

    pub fn nip05(&self) -> Option<String> {
        self.inner.nip05.clone()
    }
}
