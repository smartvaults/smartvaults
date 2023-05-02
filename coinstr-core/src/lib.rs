// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

#[cfg(all(target_arch = "wasm32", feature = "blocking"))]
compile_error!("`blocking` feature can't be enabled for WASM targets");

#[macro_use]
extern crate serde;
pub extern crate nostr_sdk;

pub use keechain_core::*;
use nostr_sdk::{nips::nip04, Keys};
use serde::{de::DeserializeOwned, Serialize};

#[cfg(feature = "cache")]
pub mod cache;
pub mod client;
pub mod constants;
#[cfg(not(target_arch = "wasm32"))]
mod keychain;
pub mod policy;
pub mod proposal;
#[cfg(feature = "reserves")]
pub mod reserves;
pub mod util;

#[cfg(feature = "blocking")]
pub use self::client::blocking::CoinstrClient;
#[cfg(not(feature = "blocking"))]
pub use self::client::CoinstrClient;
#[cfg(not(target_arch = "wasm32"))]
pub use self::keychain::Coinstr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FeeRate {
    /// High: confirm in 1 blocks
    High,
    /// Medium: confirm in 6 blocks
    #[default]
    Medium,
    /// Low: confirm in 12 blocks
    Low,
    /// Target blocks
    Custom(usize),
}

impl FeeRate {
    pub fn target_blocks(&self) -> usize {
        match self {
            Self::High => 1,
            Self::Medium => 6,
            Self::Low => 12,
            Self::Custom(target) => *target,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    Max,
    Custom(u64),
}

pub trait Encryption: Sized + Serialize + DeserializeOwned {
    /// Deserialize from `JSON` string
    fn from_json<S>(json: S) -> Result<Self, EncryptionError>
    where
        S: Into<String>,
    {
        Ok(serde_json::from_str(&json.into())?)
    }

    /// Serialize to `JSON` string
    fn as_json(&self) -> String {
        serde_json::json!(self).to_string()
    }

    /// Encrypt
    fn encrypt(&self, keys: &Keys) -> Result<String, EncryptionError> {
        Ok(nip04::encrypt(
            &keys.secret_key()?,
            &keys.public_key(),
            self.as_json(),
        )?)
    }

    /// Deccrypt
    fn decrypt<S>(keys: &Keys, content: S) -> Result<Self, EncryptionError>
    where
        S: Into<String>,
    {
        let json = nip04::decrypt(&keys.secret_key()?, &keys.public_key(), content)?;
        Self::from_json(json)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
    #[error(transparent)]
    NIP04(#[from] nostr_sdk::nips::nip04::Error),
    #[error(transparent)]
    JSON(#[from] serde_json::Error),
}
