// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Vault Invite

use nostr::{Event, EventBuilder, Keys, Tag, Timestamp};
use prost::Message;
use smartvaults_core::secp256k1::XOnlyPublicKey;

use super::Vault;
use crate::v2::constants::{WRAPPER_EXIPRATION, WRAPPER_KIND};
use crate::v2::message::{MessageVersion, ProtocolEncoding, ProtocolEncryption};
use crate::v2::proto::vault::ProtoVaultInvite;
use crate::v2::{Error, Wrapper};

/// Vault invite
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultInvite {
    /// Vault
    pub vault: Vault,
    /// Invite sender
    pub sender: Option<XOnlyPublicKey>,
    /// Invite message
    pub message: String,
}

impl VaultInvite {
    /// Compose new [Vault] invite
    pub fn new<S>(vault: Vault, sender: Option<XOnlyPublicKey>, message: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            vault,
            sender,
            message: message.into(),
        }
    }

    /// Get reference of [Vault]
    pub fn vault(&self) -> &Vault {
        &self.vault
    }

    /// Get sender
    pub fn sender(&self) -> Option<XOnlyPublicKey> {
        self.sender
    }

    /// Get message
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl ProtocolEncoding for VaultInvite {
    type Err = Error;

    fn pre_encoding(&self) -> (MessageVersion, Vec<u8>) {
        let vault: ProtoVaultInvite = self.into();
        (MessageVersion::ProtoBuf, vault.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoVaultInvite = ProtoVaultInvite::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for VaultInvite {
    type Err = Error;
}

/// Build [`Vault`] invite [`Event`]
pub fn build_event(invite: VaultInvite, receiver: XOnlyPublicKey) -> Result<Event, Error> {
    // Compose wrapper
    let wrapper: Wrapper = Wrapper::VaultInvite(invite);

    // Encrypt
    let keys = Keys::generate();
    let encrypted_content: String = wrapper.encrypt(&keys.secret_key()?, &receiver)?;

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
