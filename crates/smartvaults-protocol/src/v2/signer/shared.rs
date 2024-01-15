// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Shared Signer

use core::ops::Deref;

use nostr::{Event, EventBuilder, Keys, Tag, Timestamp};
use prost::Message;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::crypto::hash;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::CoreSigner;

use super::SignerIdentifier;
use crate::v2::constants::{SHARED_SIGNER_KIND_V2, WRAPPER_EXIPRATION, WRAPPER_KIND};
use crate::v2::message::EncodingVersion;
use crate::v2::proto::signer::ProtoSharedSigner;
use crate::v2::wrapper::Wrapper;
use crate::v2::{Error, NostrPublicIdentifier, ProtocolEncoding, ProtocolEncryption};

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

impl ProtocolEncoding for SharedSigner {
    type Err = Error;

    fn protocol_network(&self) -> Network {
        self.network()
    }

    fn pre_encoding(&self) -> (EncodingVersion, Vec<u8>) {
        let shared_signer: ProtoSharedSigner = self.into();
        (EncodingVersion::ProtoBuf, shared_signer.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let shared_signer: ProtoSharedSigner = ProtoSharedSigner::decode(data)?;
        Self::try_from(shared_signer)
    }
}

impl ProtocolEncryption for SharedSigner {
    type Err = Error;
}

/// Build [SharedSigner] invitation [`Event`]
pub fn build_invitation_event(shared_signer: &SharedSigner) -> Result<Event, Error> {
    // Compose wrapper
    let wrapper: Wrapper = Wrapper::SharedSignerInvite {
        shared_signer: shared_signer.clone(),
    };

    // Encrypt
    let keys = Keys::generate();
    let encrypted_content: String =
        wrapper.encrypt(&keys.secret_key()?, &shared_signer.receiver)?;

    // Compose and sign event
    Ok(EventBuilder::new(
        WRAPPER_KIND,
        encrypted_content,
        [
            Tag::public_key(shared_signer.receiver),
            Tag::Expiration(Timestamp::now() + WRAPPER_EXIPRATION),
        ],
    )
    .to_event(&keys)?)
}

/// Build [SharedSigner] event (used to accept an invitation)
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
