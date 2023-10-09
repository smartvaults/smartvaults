// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr::prelude::Hash;
use nostr::secp256k1::{self, SecretKey, XOnlyPublicKey};
use nostr::{key, Event, EventBuilder, Keys, Tag};
use prost::Message;
use smartvaults_core::bitcoin::Network;
use thiserror::Error;

mod proto;

use self::proto::shared_key::Object as ProtoObject;
use self::proto::{SharedKey as ProtoSharedKey, SharedKeyV1 as ProtoSharedKeyV1};
use super::constants::SHARED_KEY_KIND_V2;
use super::crypto::{self, Version as CryptoVersion};
use super::schema::{self, Schema, SchemaEncoding, SchemaVersion};
use super::{identifier, network, Identifier, NetworkMagic};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Secp256k1(#[from] secp256k1::Error),
    #[error(transparent)]
    Network(#[from] network::Error),
    #[error(transparent)]
    Crypto(#[from] crypto::Error),
    #[error(transparent)]
    Schema(#[from] schema::Error),
    #[error(transparent)]
    Identifier(#[from] identifier::Error),
    #[error(transparent)]
    Proto(#[from] prost::DecodeError),
    #[error(transparent)]
    Keys(#[from] key::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr::event::builder::Error),
    #[error("{0} not found")]
    NotFound(String),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    #[default]
    V1 = 0x01,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedKey {
    /// Version
    version: Version,
    /// Secret key
    secret_key: SecretKey,
    /// Policy Identifier
    policy_identifier: Identifier,
    /// Network magic
    network: NetworkMagic,
}

pub fn build_event(
    keys: &Keys,
    receiver: &XOnlyPublicKey,
    shared_key: &Keys,
    policy_identifier: Identifier,
    network: Network,
) -> Result<Event, Error> {
    // Compose Shared Key
    let shared_key = SharedKey {
        version: Version::default(),
        secret_key: shared_key.secret_key()?,
        policy_identifier,
        network: network.into(),
    };

    // Encrypt Shared Key
    let encrypted_shared_key = crypto::encrypt(
        &keys.secret_key()?,
        receiver,
        shared_key.encode(),
        CryptoVersion::XChaCha20Poly1305,
    )?;

    // Compose and build event
    Ok(EventBuilder::new(
        SHARED_KEY_KIND_V2,
        encrypted_shared_key,
        // Include only the public key able to decrypt the event to avoid leak of other data
        &[Tag::PubKey(*receiver, None)],
    )
    .to_event(keys)?)
}

impl SchemaEncoding for SharedKey {
    type Error = Error;

    fn decode<T>(payload: T) -> Result<Self, Self::Error>
    where
        T: AsRef<[u8]>,
    {
        let Schema { version, data } = schema::decode(payload.as_ref())?;
        match version {
            SchemaVersion::ProtoBuf => {
                let vault: ProtoSharedKey = ProtoSharedKey::decode(data)?;
                match vault.object {
                    Some(obj) => match obj {
                        ProtoObject::V1(v) => Ok(Self {
                            version: Version::V1,
                            secret_key: SecretKey::from_slice(&v.secret_key)?,
                            network: NetworkMagic::from_slice(&v.network)?,
                            policy_identifier: Identifier::from_slice(&v.policy_identifier)?,
                        }),
                    },
                    None => Err(Error::NotFound(String::from("protobuf vault obj"))),
                }
            }
        }
    }

    fn encode(&self) -> Vec<u8> {
        let vault: ProtoSharedKey = ProtoSharedKey {
            object: Some(ProtoObject::V1(ProtoSharedKeyV1 {
                secret_key: self.secret_key.secret_bytes().to_vec(),
                network: self.network.magic().to_bytes().to_vec(),
                policy_identifier: self.policy_identifier.as_byte_array().to_vec(),
            })),
        };
        let buf: Vec<u8> = vault.encode_to_vec();
        schema::encode(buf, SchemaVersion::ProtoBuf)
    }
}
