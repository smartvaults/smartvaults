// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Shared Signer

use core::ops::Deref;

use smartvaults_core::crypto::hash;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::CoreSigner;

use super::SignerIdentifier;
use crate::v2::NostrPublicIdentifier;

/// Shared Signer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedSigner {
    owner: XOnlyPublicKey,
    receiver: XOnlyPublicKey,
    core: CoreSigner,
}

impl Deref for SharedSigner {
    type Target = CoreSigner;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl SharedSigner {
    /// Compose new Shared Signer
    pub fn new(owner: XOnlyPublicKey, receiver: XOnlyPublicKey, core: CoreSigner) -> Self {
        Self {
            owner,
            receiver,
            core,
        }
    }

    /// Signer Identifier
    pub fn signer_id(&self) -> SignerIdentifier {
        SignerIdentifier::from((self.network(), self.fingerprint()))
    }

    /// The owner of the signer
    pub fn owner(&self) -> &XOnlyPublicKey {
        &self.owner
    }

    /// The receiver of the shared signer
    pub fn receiver(&self) -> &XOnlyPublicKey {
        &self.receiver
    }

    /// Generate deterministic Nostr Public Identifier
    pub fn nostr_public_identifier(&self) -> NostrPublicIdentifier {
        let unhashed: String = format!(
            "shared-signer:{}:{}:{}:{}",
            self.owner,
            self.receiver,
            self.fingerprint(),
            self.network()
        );
        NostrPublicIdentifier::from(hash::sha256(unhashed))
    }
}
