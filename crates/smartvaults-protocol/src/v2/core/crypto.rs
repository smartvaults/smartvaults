// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Versioned encryption/decryption

use chacha20poly1305::aead::{Aead, AeadCore, KeyInit, OsRng};
use chacha20poly1305::XChaCha20Poly1305;
use nostr::{util, PublicKey, SecretKey};
use smartvaults_core::bitcoin::hashes::sha256::Hash as Sha256Hash;
use smartvaults_core::bitcoin::hashes::Hash;
use smartvaults_core::util::base64;
use thiserror::Error;

/// Error
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ChaCha20Poly1305(#[from] chacha20poly1305::Error),
    /// Invalid lenght
    #[error("Invalid length")]
    InvalidLength,
    /// Error while decoding from base64
    #[error("Error while decoding from base64")]
    Base64Decode,
    /// Error while encoding to UTF-8
    #[error("Error while encoding to UTF-8")]
    Utf8Encode,
    /// Unknown version
    #[error("unknown version: {0}")]
    UnknownVersion(u8),
    /// Version not found in payload
    #[error("Version not found in payload")]
    VersionNotFound,
    /// Not found in payload
    #[error("{0} not found in payload")]
    NotFound(String),
}

/// Payload version
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Version {
    /// XChaCha20Poly1305
    XChaCha20Poly1305 = 0x00,
}

impl Version {
    /// Get [`Version`] as `u8`
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl TryFrom<u8> for Version {
    type Error = Error;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            0x00 => Ok(Self::XChaCha20Poly1305),
            v => Err(Error::UnknownVersion(v)),
        }
    }
}

/// Encrypt
pub fn encrypt<T>(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    content: T,
    version: Version,
) -> Result<String, Error>
where
    T: AsRef<[u8]>,
{
    match version {
        Version::XChaCha20Poly1305 => {
            // Compose key
            let key: [u8; 32] = util::generate_shared_key(secret_key, public_key);
            let key: Sha256Hash = Sha256Hash::hash(&key);

            // Compose cipher
            let cipher = XChaCha20Poly1305::new(&key.to_byte_array().into());

            // Generate 192-bit nonce
            let nonce = XChaCha20Poly1305::generate_nonce(&mut OsRng);

            // Encrypt
            let ciphertext: Vec<u8> = cipher.encrypt(&nonce, content.as_ref())?;

            // Compose payload
            let mut payload: Vec<u8> = Vec::with_capacity(1 + 24 + ciphertext.len());
            payload.push(version.as_u8());
            payload.extend_from_slice(nonce.as_slice());
            payload.extend(ciphertext);

            // Encode payload to base64
            Ok(base64::encode(payload))
        }
    }
}

/// Decrypt
pub fn decrypt<T>(
    secret_key: &SecretKey,
    public_key: &PublicKey,
    payload: T,
) -> Result<Vec<u8>, Error>
where
    T: AsRef<[u8]>,
{
    // Decode base64 payload
    let payload: Vec<u8> = base64::decode(payload).map_err(|_| Error::Base64Decode)?;

    // Get version byte
    let version: u8 = *payload.first().ok_or(Error::VersionNotFound)?;

    match Version::try_from(version)? {
        Version::XChaCha20Poly1305 => {
            // Get data from payload
            let nonce: &[u8] = payload
                .get(1..25)
                .ok_or_else(|| Error::NotFound(String::from("nonce")))?;
            let ciphertext: &[u8] = payload
                .get(25..)
                .ok_or_else(|| Error::NotFound(String::from("ciphertext")))?;

            // Compose key
            let key: [u8; 32] = util::generate_shared_key(secret_key, public_key);
            let key: Sha256Hash = Sha256Hash::hash(&key);

            // Compose cipher
            let cipher = XChaCha20Poly1305::new(&key.to_byte_array().into());

            // Decrypt
            Ok(cipher.decrypt(nonce.into(), ciphertext.as_ref())?)
        }
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use smartvaults_core::secp256k1::{KeyPair, Secp256k1, XOnlyPublicKey};

    use super::*;

    #[test]
    fn test_encryption_decryption() {
        let secp = Secp256k1::new();

        // Alice keys
        let alice_sk =
            SecretKey::from_str("5c0c523f52a5b6fad39ed2403092df8cebc36318b39383bca6c00808626fab3a")
                .unwrap();
        let alice_key_pair = KeyPair::from_secret_key(&secp, &alice_sk);
        let alice_pk = XOnlyPublicKey::from_keypair(&alice_key_pair).0;
        let alice_pk = PublicKey::from(alice_pk);

        // Bob keys
        let bob_sk =
            SecretKey::from_str("4b22aa260e4acb7021e32f38a6cdf4b673c6a277755bfce287e370c924dc936d")
                .unwrap();
        let bob_key_pair = KeyPair::from_secret_key(&secp, &bob_sk);
        let bob_pk = XOnlyPublicKey::from_keypair(&bob_key_pair).0;
        let bob_pk = PublicKey::from(bob_pk);

        let content = String::from("hello");

        let encrypted_content =
            encrypt(&alice_sk, &bob_pk, &content, Version::XChaCha20Poly1305).unwrap();

        assert_eq!(
            &decrypt(&bob_sk, &alice_pk, &encrypted_content).unwrap(),
            content.as_bytes()
        );
    }
}
