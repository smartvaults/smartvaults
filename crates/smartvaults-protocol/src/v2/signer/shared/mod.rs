// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Shared Signer

use core::ops::Deref;

use nostr::{Event, EventBuilder, Keys, PublicKey, Tag, Timestamp};
use prost::Message;
use smartvaults_core::crypto::hash;
use smartvaults_core::CoreSigner;

pub mod invite;

pub use self::invite::SharedSignerInvite;
use super::SignerIdentifier;
use crate::v2::constants::SHARED_SIGNER_KIND_V2;
use crate::v2::message::{MessageVersion, ProtocolEncoding, ProtocolEncryption};
use crate::v2::proto::signer::ProtoSharedSigner;
use crate::v2::{Error, NostrPublicIdentifier};

/// Shared Signer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedSigner {
    owner: PublicKey,
    receiver: PublicKey,
    core: CoreSigner,
    timestamp: Timestamp,
}

impl Deref for SharedSigner {
    type Target = CoreSigner;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl SharedSigner {
    /// Compose new Shared Signer
    pub fn new(
        owner: PublicKey,
        receiver: PublicKey,
        core: CoreSigner,
        timestamp: Timestamp,
    ) -> Self {
        Self {
            owner,
            receiver,
            core,
            timestamp,
        }
    }

    /// Signer Identifier
    pub fn signer_id(&self) -> SignerIdentifier {
        SignerIdentifier::from((self.network(), self.fingerprint()))
    }

    /// The owner of the signer
    pub fn owner(&self) -> &PublicKey {
        &self.owner
    }

    /// The receiver of the shared signer
    pub fn receiver(&self) -> &PublicKey {
        &self.receiver
    }

    /// Timestamp of the shared signer
    ///
    /// Needed to allow to replace an already shared signer with a new version
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Consume [SharedSigner] and create invite
    pub fn to_invite<S>(self, message: S) -> SharedSignerInvite
    where
        S: Into<String>,
    {
        let sender: PublicKey = self.owner;
        SharedSignerInvite::new(self, Some(sender), message)
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

impl ProtocolEncoding for SharedSigner {
    type Err = Error;

    fn pre_encoding(&self) -> (MessageVersion, Vec<u8>) {
        let shared_signer: ProtoSharedSigner = self.into();
        (MessageVersion::ProtoBuf, shared_signer.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let shared_signer: ProtoSharedSigner = ProtoSharedSigner::decode(data)?;
        Self::try_from(shared_signer)
    }
}

impl ProtocolEncryption for SharedSigner {
    type Err = Error;
}

/// Build [SharedSigner] event
///
/// Must use **own** [`Keys`] (not random or shared key)!
pub fn build_event(keys: &Keys, shared_signer: &SharedSigner) -> Result<Event, Error> {
    // Encrypt
    let encrypted_content: String = shared_signer.encrypt_with_keys(keys)?;

    // Compose and build event
    let identifier: String = shared_signer.nostr_public_identifier().to_string();
    Ok(EventBuilder::new(
        SHARED_SIGNER_KIND_V2,
        encrypted_content,
        [Tag::Identifier(identifier)],
    )
    .to_event(keys)?)
}
