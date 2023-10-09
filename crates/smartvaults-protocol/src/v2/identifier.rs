// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;

use smartvaults_core::bitcoin::hashes::sha256::Hash as Sha256Hash;
use smartvaults_core::bitcoin::hashes::{self, Hash};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Hash(#[from] hashes::Error),
}

/// Deterministic identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Identifier {
    inner: Sha256Hash,
}

impl Deref for Identifier {
    type Target = Sha256Hash;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl Identifier {
    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        Ok(Self {
            inner: Sha256Hash::from_slice(slice)?,
        })
    }
}
