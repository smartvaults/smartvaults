// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Proposal Identifier

use core::fmt;
use core::ops::Deref;

use smartvaults_core::bitcoin::{Network, Transaction};
use smartvaults_core::crypto::hash;
use smartvaults_core::hashes::sha256::Hash as Sha256Hash;
use smartvaults_core::hashes::Hash;

use crate::v2::Error;

/// Proposal Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProposalIdentifier(Sha256Hash);

impl Deref for ProposalIdentifier {
    type Target = Sha256Hash;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<(Network, &Transaction)> for ProposalIdentifier {
    fn from((network, tx): (Network, &Transaction)) -> Self {
        let unhashed_identifier: String = format!("{}:{}", network.magic(), tx.txid());
        Self(hash::sha256(unhashed_identifier))
    }
}

impl fmt::Display for ProposalIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl ProposalIdentifier {
    /// Compose vault identifier from bytes
    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        Ok(Self(Sha256Hash::from_slice(slice)?))
    }
}
