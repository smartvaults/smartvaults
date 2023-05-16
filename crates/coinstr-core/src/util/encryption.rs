// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use nostr_sdk::nips::nip04;
use nostr_sdk::Keys;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
    #[error(transparent)]
    NIP04(#[from] nostr_sdk::nips::nip04::Error),
    #[error(transparent)]
    JSON(#[from] serde_json::Error),
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
