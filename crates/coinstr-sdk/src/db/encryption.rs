// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use core::fmt;

use chacha20poly1305::aead::{Aead, OsRng};
use chacha20poly1305::{AeadCore, XChaCha20Poly1305};
use coinstr_core::bdk::wallet::ChangeSet;
use coinstr_core::secp256k1::SecretKey;
use coinstr_core::util::serde::deserialize;
use coinstr_core::{ApprovedProposal, CompletedProposal, Policy, Proposal, SharedSigner, Signer};
use coinstr_protocol::v1::{Label, Serde};

/// Error
#[derive(Debug, PartialEq, Eq)]
pub enum Error {
    /// ChaCha20Poly1305 error
    ChaCha20Poly1305(chacha20poly1305::Error),
    // Json error
    Json(String),
    /// Not found in payload
    NotFound(String),
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ChaCha20Poly1305(e) => write!(f, "ChaCha20Poly1305: {e}"),
            Self::Json(e) => write!(f, "Json: {e}"),
            Self::NotFound(value) => write!(f, "{value} not found in payload"),
        }
    }
}

impl From<chacha20poly1305::Error> for Error {
    fn from(e: chacha20poly1305::Error) -> Self {
        Self::ChaCha20Poly1305(e)
    }
}

pub trait StoreEncryption: Serde {
    /// Encrypt
    fn encrypt(&self, cipher: &XChaCha20Poly1305) -> Result<Vec<u8>, Error> {
        // Serialize to JSON
        let json = self.as_json();

        // Generate 192-bit nonce
        let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

        // Encrypt
        let ciphertext: Vec<u8> = cipher.encrypt(&nonce, json.as_bytes())?;

        // Compose payload
        let mut payload: Vec<u8> = Vec::new();
        payload.extend_from_slice(nonce.as_slice());
        payload.extend(ciphertext);

        Ok(payload)
    }

    /// Decrypt
    fn decrypt<T>(cipher: &XChaCha20Poly1305, content: T) -> Result<Self, Error>
    where
        T: AsRef<[u8]>,
    {
        let payload: &[u8] = content.as_ref();

        // Get data from payload
        let nonce: &[u8] = payload
            .get(0..24)
            .ok_or_else(|| Error::NotFound(String::from("nonce")))?;
        let ciphertext: &[u8] = payload
            .get(24..)
            .ok_or_else(|| Error::NotFound(String::from("ciphertext")))?;

        // Decrypt
        let data: Vec<u8> = cipher.decrypt(nonce.into(), ciphertext.as_ref())?;

        deserialize(data).map_err(|e| Error::Json(e.to_string()))
    }
}

impl StoreEncryption for SecretKey {}
impl StoreEncryption for ChangeSet {}
impl StoreEncryption for Policy {}
impl StoreEncryption for Proposal {}
impl StoreEncryption for ApprovedProposal {}
impl StoreEncryption for CompletedProposal {}
impl StoreEncryption for Signer {}
impl StoreEncryption for SharedSigner {}
impl StoreEncryption for Label {}
