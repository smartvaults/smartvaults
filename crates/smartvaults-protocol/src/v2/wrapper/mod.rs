// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use prost::Message;
use thiserror::Error;

use super::core::{CryptoError, SchemaError, SchemaVersion};
use super::proto::wrapper::{ProtoWrapper, ProtoWrapperObject};
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
    #[error("{0} not found")]
    NotFound(String),
}

pub enum Wrapper {
    VaultInvite { vault: Vault },
}

impl ProtocolEncoding for Wrapper {
    type Err = Error;

    fn encode_pre_schema(&self) -> (SchemaVersion, Vec<u8>) {
        let wrapper: ProtoWrapper = ProtoWrapper {
            object: Some(match self {
                Self::VaultInvite { vault } => ProtoWrapperObject::Vault(vault.into()),
            }),
        };
        (SchemaVersion::ProtoBuf, wrapper.encode_to_vec())
    }

    fn decode_proto(_data: &[u8]) -> Result<Self, Self::Err> {
        todo!()
    }
}

impl ProtocolEncryption for Wrapper {
    type Err = Error;
}
