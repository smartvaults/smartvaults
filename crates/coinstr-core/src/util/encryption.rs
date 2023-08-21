// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use keechain_core::bdk::wallet::ChangeSet;
use keechain_core::bitcoin::secp256k1::SecretKey;
use keechain_core::crypto::aes;
use keechain_core::util::serde::deserialize;

use super::serde::Serde;

#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    #[error(transparent)]
    Aes(#[from] aes::Error),
    #[error(transparent)]
    JSON(#[from] serde_json::Error),
}

pub trait Encryption: Serde {
    /// Encrypt
    fn encrypt(&self, key: [u8; 32]) -> String {
        aes::encrypt(key, self.as_json())
    }

    /// Decrypt
    fn decrypt<T>(key: [u8; 32], content: T) -> Result<Self, EncryptionError>
    where
        T: AsRef<[u8]>,
    {
        let data: Vec<u8> = aes::decrypt(key, content)?;
        Ok(deserialize(data)?)
    }
}

impl Serde for SecretKey {}
impl Encryption for SecretKey {}

impl Serde for ChangeSet {}
impl Encryption for ChangeSet {}
