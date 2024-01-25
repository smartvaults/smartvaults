// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Vault metadata

use nostr::{Event, EventBuilder, Keys};
use prost::Message;

use super::{Vault, VaultIdentifier};
use crate::v2::constants::VAULT_METADATA_KIND_V2;
use crate::v2::message::{MessageVersion, ProtocolEncoding, ProtocolEncryption};
use crate::v2::proto::vault::ProtoVaultMetadata;
use crate::v2::Error;

/// Vault metadata
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultMetadata {
    vault_id: VaultIdentifier,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
}

impl VaultMetadata {
    /// New empty vault metadata
    pub fn new(vault_id: VaultIdentifier) -> Self {
        Self {
            vault_id,
            name: String::new(),
            description: String::new(),
        }
    }

    /// Vault Identifier
    pub fn vault_id(&self) -> VaultIdentifier {
        self.vault_id
    }

    /// Change vault metadata name
    pub fn change_name<S>(&mut self, name: S)
    where
        S: Into<String>,
    {
        self.name = name.into();
    }

    /// Change vault metadata description
    pub fn change_description<S>(&mut self, description: S)
    where
        S: Into<String>,
    {
        self.description = description.into();
    }
}

impl ProtocolEncoding for VaultMetadata {
    type Err = Error;

    fn pre_encoding(&self) -> (MessageVersion, Vec<u8>) {
        let vault: ProtoVaultMetadata = self.into();
        (MessageVersion::ProtoBuf, vault.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoVaultMetadata = ProtoVaultMetadata::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for VaultMetadata {
    type Err = Error;
}

/// Build [VaultMetadata] event
pub fn build_event(vault: &Vault, metadata: &VaultMetadata) -> Result<Event, Error> {
    // Keys
    let keys: Keys = Keys::new(vault.shared_key());

    // Encrypt
    let encrypted_content: String = metadata.encrypt_with_keys(&keys)?;

    // Compose and build event
    Ok(EventBuilder::new(VAULT_METADATA_KIND_V2, encrypted_content, []).to_event(&keys)?)
}
