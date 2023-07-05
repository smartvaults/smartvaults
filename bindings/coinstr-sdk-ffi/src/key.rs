// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::nostr::key::{self};
use coinstr_sdk::nostr::nips::nip19::ToBech32;

use crate::error::Result;

#[derive(Clone)]
pub struct Keys {
    inner: key::Keys,
}

impl From<key::Keys> for Keys {
    fn from(inner: key::Keys) -> Self {
        Self { inner }
    }
}

impl Keys {
    pub fn public_key(&self) -> String {
        self.inner.public_key().to_string()
    }

    pub fn public_key_bech32(&self) -> Result<String> {
        Ok(self.inner.public_key().to_bech32()?)
    }

    pub fn secret_key(&self) -> Result<String> {
        Ok(self.inner.secret_key()?.display_secret().to_string())
    }

    pub fn secret_key_bech32(&self) -> Result<String> {
        Ok(self.inner.secret_key()?.to_bech32()?)
    }
}
