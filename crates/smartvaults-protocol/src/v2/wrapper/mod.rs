// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use prost::Message;
use thiserror::Error;

use super::core::{CryptoError, SchemaError, SchemaVersion};
use super::proto::wrapper::{ProtoWrapper, ProtoWrapperObject};
use super::vault::Error as VaultError;
use super::{ProtocolEncoding, ProtocolEncryption, Vault};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error(transparent)]
    Proto(#[from] prost::DecodeError),
    #[error(transparent)]
    Keys(#[from] nostr::key::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr::event::builder::Error),
    #[error(transparent)]
    Vault(#[from] VaultError),
    #[error("{0} not found")]
    NotFound(String),
}

pub enum Wrapper {
    VaultInvite { vault: Vault },
}

impl ProtocolEncoding for Wrapper {
    type Err = Error;

    fn pre_encoding(&self) -> (SchemaVersion, Vec<u8>) {
        let wrapper: ProtoWrapper = ProtoWrapper {
            object: Some(match self {
                Self::VaultInvite { vault } => ProtoWrapperObject::Vault(vault.into()),
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
            },
            None => Err(Error::NotFound(String::from("protobuf wrapper obj"))),
        }
    }
}

impl ProtocolEncryption for Wrapper {
    type Err = Error;
}
