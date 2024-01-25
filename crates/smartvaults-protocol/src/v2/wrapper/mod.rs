// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Wrapper
//!
//! The Wrapper is used to send data without leaking metadata to the public

use prost::Message;
use smartvaults_core::secp256k1::XOnlyPublicKey;

mod proto;

use super::proto::wrapper::ProtoWrapper;
use super::vault::VaultInvite;
use super::{Error, SharedSigner};
use crate::v2::message::{EncodingVersion, ProtocolEncoding, ProtocolEncryption};

/// Smart Vaults Wrapper
pub enum Wrapper {
    /// Vault invite
    VaultInvite(VaultInvite),
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
