// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Shared Signer Invite

use core::cmp::Ordering;
use core::hash::{Hash, Hasher};

use nostr::{Event, EventBuilder, Keys, PublicKey, Tag, Timestamp};
use prost::Message;

use super::SharedSigner;
use crate::v2::constants::{WRAPPER_EXIPRATION, WRAPPER_KIND};
use crate::v2::message::{MessageVersion, ProtocolEncoding, ProtocolEncryption};
use crate::v2::proto::signer::ProtoSharedSignerInvite;
use crate::v2::{Error, Wrapper};

/// Shared Signer invite
#[derive(Debug, Clone)]
pub struct SharedSignerInvite {
    /// Shared Signer
    pub shared_signer: SharedSigner,
    /// Invite sender
    pub sender: Option<PublicKey>,
    /// Invite message
    pub message: String,
    /// Invite timestamp
    pub timestamp: Timestamp,
}

impl PartialEq for SharedSignerInvite {
    fn eq(&self, other: &Self) -> bool {
        self.shared_signer == other.shared_signer
    }
}

impl Eq for SharedSignerInvite {}

impl PartialOrd for SharedSignerInvite {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SharedSignerInvite {
    fn cmp(&self, other: &Self) -> Ordering {
        self.shared_signer.cmp(&other.shared_signer)
    }
}

impl Hash for SharedSignerInvite {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.shared_signer.hash(state);
    }
}

impl SharedSignerInvite {
    /// Compose new [SharedSigner] invite
    pub fn new<S>(shared_signer: SharedSigner, sender: Option<PublicKey>, message: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            shared_signer,
            sender,
            message: message.into(),
            timestamp: Timestamp::now(),
        }
    }

    /// Get reference of [SharedSigner]
    pub fn shared_signer(&self) -> &SharedSigner {
        &self.shared_signer
    }

    /// Get sender
    pub fn sender(&self) -> Option<PublicKey> {
        self.sender
    }

    /// Get message
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Get timestamp
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }
}

impl ProtocolEncoding for SharedSignerInvite {
    type Err = Error;

    fn pre_encoding(&self) -> (MessageVersion, Vec<u8>) {
        let vault: ProtoSharedSignerInvite = self.into();
        (MessageVersion::ProtoBuf, vault.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoSharedSignerInvite = ProtoSharedSignerInvite::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for SharedSignerInvite {
    type Err = Error;
}

/// Build [`SharedSigner`] invite [`Event`]
pub fn build_event(invite: SharedSignerInvite, receiver: PublicKey) -> Result<Event, Error> {
    // Compose wrapper
    let wrapper: Wrapper = Wrapper::SharedSignerInvite(invite);

    // Encrypt
    let keys = Keys::generate();
    let encrypted_content: String = wrapper.encrypt(keys.secret_key()?, &receiver)?;

    // Compose and sign event
    Ok(EventBuilder::new(
        WRAPPER_KIND,
        encrypted_content,
        [
            Tag::public_key(receiver),
            Tag::Expiration(Timestamp::now() + WRAPPER_EXIPRATION),
        ],
    )
    .to_event(&keys)?)
}
