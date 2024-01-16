// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Wrapper
//!
//! The Wrapper is used to send data without leaking metadata to the public

use prost::Message;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::secp256k1::XOnlyPublicKey;

mod proto;

use super::message::EncodingVersion;
use super::proto::wrapper::ProtoWrapper;
use super::{Error, ProtocolEncoding, ProtocolEncryption, SharedSigner, Vault};

/// Smart Vaults Wrapper
pub enum Wrapper {
    /// Vault invite
    VaultInvite {
        /// Vault
        vault: Vault,
        /// Invite sender
        sender: Option<XOnlyPublicKey>,
    },
    /// Shared Signer invite
    SharedSignerInvite {
        /// Shared Signer
        shared_signer: SharedSigner,
        /// Invite sender
        sender: Option<XOnlyPublicKey>,
    },
}

impl ProtocolEncoding for Wrapper {
    type Err = Error;

    fn protocol_network(&self) -> Network {
        match self {
            Self::VaultInvite { vault, .. } => vault.network(),
            Self::SharedSignerInvite { shared_signer, .. } => shared_signer.network(),
        }
    }

    fn pre_encoding(&self) -> (EncodingVersion, Vec<u8>) {
        let wrapper: ProtoWrapper = self.into();
        (EncodingVersion::ProtoBuf, wrapper.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let wrapper: ProtoWrapper = ProtoWrapper::decode(data)?;
        Self::try_from(wrapper)
    }
}

impl ProtocolEncryption for Wrapper {
    type Err = Error;
}
