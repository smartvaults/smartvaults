// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Signer Identifier

use core::fmt;
use core::ops::Deref;
use core::str::FromStr;

use smartvaults_core::bitcoin::bip32::Fingerprint;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::crypto::hash;
use smartvaults_core::hashes::sha256::Hash as Sha256Hash;
use smartvaults_core::hashes::Hash;

use crate::v2::Error;

/// Signer Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SignerIdentifier(Sha256Hash);

impl Deref for SignerIdentifier {
    type Target = Sha256Hash;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<(Network, Fingerprint)> for SignerIdentifier {
    fn from((network, fingerprint): (Network, Fingerprint)) -> Self {
        let unhashed: String = format!("{}:{fingerprint}", network.magic());
        Self(hash::sha256(unhashed))
    }
}

impl fmt::Display for SignerIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl SignerIdentifier {
    /// Compose vault identifier from bytes
    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        Ok(Self(Sha256Hash::from_slice(slice)?))
    }
}

impl FromStr for SignerIdentifier {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Sha256Hash::from_str(s)?))
    }
}
