// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;
use core::str::FromStr;

use nostr::{Event, EventBuilder, Keys, Tag};
use prost::Message;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::{self, Descriptor};
use smartvaults_core::policy::{self, Policy};
use smartvaults_core::PolicyTemplate;
use thiserror::Error;

pub mod metadata;
mod proto;

pub use self::metadata::VaultMetadata;
use self::proto::vault::Object as ProtoObject;
use self::proto::{Vault as ProtoVault, VaultV1 as ProtoVaultV1};
use super::constants::VAULT_KIND_V2;
use super::core::{
    schema, CryptoError, ProtocolEncoding, ProtocolEncryption, Schema, SchemaError, SchemaVersion,
};
use super::network::{self, NetworkMagic};
use super::SharedKey;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Policy(#[from] policy::Error),
    #[error(transparent)]
    Network(#[from] network::Error),
    #[error(transparent)]
    Miniscript(#[from] miniscript::Error),
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
    #[error("network not match: expected={expected}, found={found}")]
    NetworkNotMatch { expected: Network, found: Network },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    #[default]
    V1 = 0x01,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vault {
    version: Version,
    policy: Policy,
}

impl Deref for Vault {
    type Target = Policy;

    fn deref(&self) -> &Self::Target {
        &self.policy
    }
}

impl Vault {
    pub fn new<S, P>(
        _name: S,
        _description: S,
        descriptor: P,
        network: Network,
    ) -> Result<Self, policy::Error>
    where
        S: Into<String>,
        P: Into<String>,
    {
        let policy: Policy = Policy::from_desc_or_miniscript(descriptor, network)?;
        Ok(Self {
            version: Version::default(),
            policy,
        })
    }

    pub fn from_template<S>(
        _name: S,
        _description: S,
        template: PolicyTemplate,
        network: Network,
    ) -> Result<Self, policy::Error>
    where
        S: Into<String>,
    {
        let policy: Policy = Policy::from_template(template, network)?;
        Ok(Self {
            version: Version::default(),
            policy,
        })
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn policy(&self) -> Policy {
        self.policy.clone()
    }
}

impl ProtocolEncoding for Vault {
    type Err = Error;

    fn decode<T>(payload: T) -> Result<Self, Self::Err>
    where
        T: AsRef<[u8]>,
    {
        let Schema { version, data } = schema::decode(payload.as_ref())?;
        match version {
            SchemaVersion::ProtoBuf => {
                let vault: ProtoVault = ProtoVault::decode(data)?;
                match vault.object {
                    Some(obj) => match obj {
                        ProtoObject::V1(v) => {
                            let descriptor: Descriptor<String> =
                                Descriptor::from_str(&v.descriptor)?;
                            let network: NetworkMagic = NetworkMagic::from_slice(&v.network)?;
                            Ok(Self {
                                version: Version::V1,
                                policy: Policy::new(descriptor, *network)?,
                            })
                        }
                    },
                    None => Err(Error::NotFound(String::from("protobuf vault obj"))),
                }
            }
        }
    }

    fn encode(&self) -> Vec<u8> {
        let vault: ProtoVault = ProtoVault {
            object: Some(ProtoObject::V1(ProtoVaultV1 {
                descriptor: self.as_descriptor().to_string(),
                network: self.network().magic().to_bytes().to_vec(),
            })),
        };
        let buf: Vec<u8> = vault.encode_to_vec();
        schema::encode(buf, SchemaVersion::ProtoBuf)
    }
}

impl ProtocolEncryption for Vault {
    type Err = Error;
}

pub fn build_event(vault: &Vault, shared_key: &SharedKey) -> Result<Event, Error> {
    if vault.network() != shared_key.network() {
        return Err(Error::NetworkNotMatch {
            expected: vault.network(),
            found: shared_key.network(),
        });
    }

    // Encrypt Shared Key
    let keys: Keys = Keys::new(shared_key.secret_key());
    let encrypted_content: String = vault.encrypt_with_keys(&keys)?;

    // Compose and build event
    Ok(EventBuilder::new(
        VAULT_KIND_V2,
        encrypted_content,
        &[Tag::Identifier("TODO".into())], // TODO: tags and identifier
    )
    .to_event(&keys)?)
}
