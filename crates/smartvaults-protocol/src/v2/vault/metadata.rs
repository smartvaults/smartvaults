// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Vault metadata

use prost::Message;
use smartvaults_core::bitcoin::Network;

use super::VaultIdentifier;
use crate::v2::message::EncodingVersion;
use crate::v2::proto::vault::ProtoVaultMetadata;
use crate::v2::{Error, ProtocolEncoding};

/// Vault metadata
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultMetadata {
    vault_id: VaultIdentifier,
    network: Network,
    /// Name
    pub name: String,
    /// Description
    pub description: String,
}

impl VaultMetadata {
    /// New empty vault metadata
    pub fn new(vault_id: VaultIdentifier, network: Network) -> Self {
        Self {
            vault_id,
            network,
            name: String::new(),
            description: String::new(),
        }
    }

    /// Vault Identifier
    pub fn vault_id(&self) -> VaultIdentifier {
        self.vault_id
    }

    /// Network
    pub fn network(&self) -> Network {
        self.network
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

    fn protocol_network(&self) -> Network {
        self.network
    }

    fn pre_encoding(&self) -> (EncodingVersion, Vec<u8>) {
        let vault: ProtoVaultMetadata = self.into();
        (EncodingVersion::ProtoBuf, vault.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoVaultMetadata = ProtoVaultMetadata::decode(data)?;
        Self::try_from(vault)
    }
}
