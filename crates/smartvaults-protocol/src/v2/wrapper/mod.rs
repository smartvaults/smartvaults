// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Wrapper
//!
//! The Wrapper is used to send data without leaking metadata to the public

use prost::Message;

use super::core::SchemaVersion;
use super::proto::wrapper::{ProtoWrapper, ProtoWrapperObject};
use super::{Error, ProtocolEncoding, ProtocolEncryption, SharedSigner, Vault};

/// Smart Vaults Wrapper
pub enum Wrapper {
    /// Vault invite
    VaultInvite {
        /// Vault
        vault: Vault,
    },
    /// Shared Signer invite
    SharedSignerInvite {
        /// Shared Signer
        shared_signer: SharedSigner,
    },
}

impl ProtocolEncoding for Wrapper {
    type Err = Error;

    fn pre_encoding(&self) -> (SchemaVersion, Vec<u8>) {
        let wrapper: ProtoWrapper = ProtoWrapper {
            object: Some(match self {
                Self::VaultInvite { vault } => ProtoWrapperObject::Vault(vault.into()),
                Self::SharedSignerInvite { shared_signer } => {
                    ProtoWrapperObject::SharedSigner(shared_signer.into())
                }
            }),
        };
        (SchemaVersion::ProtoBuf, wrapper.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let wrapper: ProtoWrapper = ProtoWrapper::decode(data)?;
        match wrapper.object {
            Some(obj) => match obj {
                ProtoWrapperObject::Vault(vault) => Ok(Self::VaultInvite {
                    vault: Vault::try_from(vault)?,
                }),
                ProtoWrapperObject::SharedSigner(shared_signer) => Ok(Self::SharedSignerInvite {
                    shared_signer: SharedSigner::try_from(shared_signer)?,
                }),
            },
            None => Err(Error::NotFound(String::from("protobuf wrapper obj"))),
        }
    }
}

impl ProtocolEncryption for Wrapper {
    type Err = Error;
}
