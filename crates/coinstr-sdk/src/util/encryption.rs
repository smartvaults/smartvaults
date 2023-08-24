// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bdk::wallet::ChangeSet;
use coinstr_core::secp256k1;
use coinstr_core::signer::{SharedSigner, Signer};
use coinstr_core::util::{Encryption, EncryptionError};
use coinstr_core::{ApprovedProposal, CompletedProposal, Policy, Proposal};
use nostr_sdk::key::{self, Keys};
use nostr_sdk::nostr::util;
use nostr_sdk::secp256k1::SecretKey;

#[derive(Debug, thiserror::Error)]
pub enum EncryptionWithKeysError {
    #[error(transparent)]
    Encryption(#[from] EncryptionError),
    #[error(transparent)]
    Keys(#[from] key::Error),
    #[error(transparent)]
    Secp256k1(#[from] secp256k1::Error),
}

pub trait EncryptionWithKeys: Encryption {
    /// Encrypt
    fn encrypt_with_keys(&self, keys: &Keys) -> Result<String, EncryptionWithKeysError> {
        let key: [u8; 32] = util::generate_shared_key(&keys.secret_key()?, &keys.public_key())?;
        Ok(self.encrypt(key))
    }

    /// Decrypt
    fn decrypt_with_keys<T>(keys: &Keys, content: T) -> Result<Self, EncryptionWithKeysError>
    where
        T: AsRef<[u8]>,
    {
        let key: [u8; 32] = util::generate_shared_key(&keys.secret_key()?, &keys.public_key())?;
        Ok(Self::decrypt(key, content)?)
    }
}

impl EncryptionWithKeys for SecretKey {}
impl EncryptionWithKeys for Policy {}
impl EncryptionWithKeys for Proposal {}
impl EncryptionWithKeys for ApprovedProposal {}
impl EncryptionWithKeys for CompletedProposal {}
impl EncryptionWithKeys for Signer {}
impl EncryptionWithKeys for SharedSigner {}
impl EncryptionWithKeys for ChangeSet {}
