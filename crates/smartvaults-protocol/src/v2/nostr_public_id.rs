// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Nostr Public Identifier

use core::fmt;

use smartvaults_core::hashes::sha256::Hash as Sha256Hash;
use smartvaults_core::hashes::Hash;
use smartvaults_core::util::hex;

const NOSTR_PUBLIC_IDENTIFIER_SIZE: usize = 12;

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
