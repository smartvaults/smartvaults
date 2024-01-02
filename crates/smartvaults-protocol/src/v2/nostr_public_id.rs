// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Nostr Public Identifier

use core::fmt;
use core::str::FromStr;

use smartvaults_core::hashes::sha256::Hash as Sha256Hash;
use smartvaults_core::hashes::Hash;
use smartvaults_core::util::hex;

use crate::v2::Error;

const NOSTR_PUBLIC_IDENTIFIER_SIZE: usize = 16;

/// Nostr Public Identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NostrPublicIdentifier([u8; NOSTR_PUBLIC_IDENTIFIER_SIZE]);

impl fmt::Display for NostrPublicIdentifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl From<Sha256Hash> for NostrPublicIdentifier {
    fn from(h: Sha256Hash) -> Self {
        let mut id: [u8; NOSTR_PUBLIC_IDENTIFIER_SIZE] = [0u8; NOSTR_PUBLIC_IDENTIFIER_SIZE];
        let cutted_hash: &[u8] = &h.to_byte_array()[..NOSTR_PUBLIC_IDENTIFIER_SIZE];
        id.copy_from_slice(cutted_hash);
        Self(id)
    }
}

impl FromStr for NostrPublicIdentifier {
    type Err = Error;
    fn from_str(id: &str) -> Result<Self, Self::Err> {
        let decode: Vec<u8> = hex::decode(id)?;
        Ok(Self(decode.as_slice().try_into()?))
    }
}
