// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Wrapper
//!
//! The Wrapper is used to send data without leaking metadata to the public

use prost::Message;

mod proto;

use super::proto::wrapper::ProtoWrapper;
use super::signer::shared::invite::SharedSignerInvite;
use super::vault::VaultInvite;
use super::Error;
use crate::v2::message::{MessageVersion, ProtocolEncoding, ProtocolEncryption};

/// Smart Vaults Wrapper
pub enum Wrapper {
    /// Vault invite
    VaultInvite(VaultInvite),
    // VaultInviteAccepted
    /// Shared Signer invite
    SharedSignerInvite(SharedSignerInvite),
}

impl ProtocolEncoding for Wrapper {
    type Err = Error;

    fn pre_encoding(&self) -> (MessageVersion, Vec<u8>) {
        let wrapper: ProtoWrapper = self.into();
        (MessageVersion::ProtoBuf, wrapper.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let wrapper: ProtoWrapper = ProtoWrapper::decode(data)?;
        Self::try_from(wrapper)
    }
}

impl ProtocolEncryption for Wrapper {
    type Err = Error;
}
