// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::types;
use nostr_sdk_ffi::{Metadata, PublicKey};

pub struct User {
    inner: types::User,
}

impl From<types::User> for User {
    fn from(inner: types::User) -> Self {
        Self { inner }
    }
}

impl User {
    pub fn public_key(&self) -> Arc<PublicKey> {
        Arc::new(self.inner.public_key().into())
    }

    pub fn metadata(&self) -> Arc<Metadata> {
        Arc::new(self.inner.metadata().into())
    }

    pub fn name(&self) -> String {
        self.inner.name()
    }
}
