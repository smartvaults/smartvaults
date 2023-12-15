// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr::{key, util, Keys};
use smartvaults_core::bdk::wallet::ChangeSet;
use smartvaults_core::bitcoin::secp256k1::SecretKey;
use smartvaults_core::crypto::aes;
use smartvaults_core::secp256k1;
use smartvaults_core::util::serde::deserialize;
use smartvaults_core::{
    ApprovedProposal, CompletedProposal, Policy, Proposal, SharedSigner, Signer,
};

use super::serde::Serde;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Aes(#[from] aes::Error),
    #[error(transparent)]
    JSON(#[from] serde_json::Error),
    #[error(transparent)]
    Keys(#[from] key::Error),
    #[error(transparent)]
    Secp256k1(#[from] secp256k1::Error),
}

pub trait Encryption: Serde {
    /// Encrypt
    fn encrypt(&self, key: [u8; 32]) -> String {
        aes::encrypt(key, self.as_json())
    }

    /// Decrypt
    fn decrypt<T>(key: [u8; 32], content: T) -> Result<Self, Error>
    where
        T: AsRef<[u8]>,
    {
        let data: Vec<u8> = aes::decrypt(key, content)?;
        Ok(deserialize(data)?)
    }

    /// Encrypt
    fn encrypt_with_keys(&self, keys: &Keys) -> Result<String, Error> {
        let key: [u8; 32] = util::generate_shared_key(&keys.secret_key()?, &keys.public_key());
        Ok(self.encrypt(key))
    }

    /// Decrypt
    fn decrypt_with_keys<T>(keys: &Keys, content: T) -> Result<Self, Error>
    where
        T: AsRef<[u8]>,
    {
        let key: [u8; 32] = util::generate_shared_key(&keys.secret_key()?, &keys.public_key());
        Self::decrypt(key, content)
    }
}

impl Serde for SecretKey {}
impl Encryption for SecretKey {}

impl Serde for ChangeSet {}

impl Serde for Policy {}
impl Encryption for Policy {}

impl Serde for Proposal {}
impl Encryption for Proposal {}

impl Serde for ApprovedProposal {}
impl Encryption for ApprovedProposal {}

impl Serde for CompletedProposal {}
impl Encryption for CompletedProposal {}

impl Serde for Signer {}
impl Encryption for Signer {}

impl Serde for SharedSigner {}
impl Encryption for SharedSigner {}
