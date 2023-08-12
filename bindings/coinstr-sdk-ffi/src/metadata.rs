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

    pub fn about(&self) -> Option<String> {
        self.inner.about.clone()
    }

    pub fn website(&self) -> Option<String> {
        self.inner.website.clone()
    }

    pub fn picture(&self) -> Option<String> {
        self.inner.picture.clone()
    }

    pub fn banner(&self) -> Option<String> {
        self.inner.banner.clone()
    }

    pub fn nip05(&self) -> Option<String> {
        self.inner.nip05.clone()
    }

    pub fn lud06(&self) -> Option<String> {
        self.inner.lud06.clone()
    }

    pub fn lud16(&self) -> Option<String> {
        self.inner.lud16.clone()
    }
}
