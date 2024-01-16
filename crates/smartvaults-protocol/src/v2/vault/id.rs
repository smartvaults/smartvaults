// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Vault Identifier

use core::fmt;
use core::ops::Deref;
use core::str::FromStr;

use smartvaults_core::crypto::hash;
use smartvaults_core::hashes::sha256::Hash as Sha256Hash;
use smartvaults_core::hashes::Hash;
use smartvaults_core::miniscript::{Descriptor, MiniscriptKey};

use crate::v2::Error;

/// Vault Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultIdentifier(Sha256Hash);

impl Deref for VaultIdentifier {
    type Target = Sha256Hash;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<Pk> From<&Descriptor<Pk>> for VaultIdentifier
where
    Pk: MiniscriptKey,
{
    fn from(descriptor: &Descriptor<Pk>) -> Self {
        Self(hash::sha256(descriptor.to_string()))
    }
}

impl fmt::Display for VaultIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl VaultIdentifier {
    /// Compose vault identifier from bytes
    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        Ok(Self(Sha256Hash::from_slice(slice)?))
    }
}

impl FromStr for VaultIdentifier {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Sha256Hash::from_str(s)?))
    }
}
