// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;
use core::str::FromStr;

use nostr::{Event, EventBuilder, Keys, Tag, Timestamp};
use prost::Message;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::{self, Descriptor};
use smartvaults_core::policy::{self, Policy};
use smartvaults_core::secp256k1::{self, SecretKey, XOnlyPublicKey};
use smartvaults_core::PolicyTemplate;
use thiserror::Error;

pub mod metadata;

pub use self::metadata::VaultMetadata;
use super::constants::{VAULT_KIND_V2, WRAPPER_EXIPRATION, WRAPPER_KIND};
use super::core::{CryptoError, ProtocolEncoding, ProtocolEncryption, SchemaError, SchemaVersion};
use super::network::{self, NetworkMagic};
use super::proto::vault::{ProtoVault, ProtoVaultObject, ProtoVaultV1};
use super::wrapper::Wrapper;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Secp256k1(#[from] secp256k1::Error),
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
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    #[default]
    V1 = 0x01,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vault {
    version: Version,
    policy: Policy,
    shared_key: SecretKey,
}

impl Deref for Vault {
    type Target = Policy;

    fn deref(&self) -> &Self::Target {
        &self.policy
    }
}

impl Vault {
    pub fn new<S>(descriptor: S, network: Network, shared_key: SecretKey) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        Ok(Self {
            version: Version::default(),
            policy: Policy::from_desc_or_miniscript(descriptor, network)?,
            shared_key,
        })
    }

    pub fn from_template<S>(
        template: PolicyTemplate,
        network: Network,
        shared_key: SecretKey,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        Ok(Self {
            version: Version::default(),
            policy: Policy::from_template(template, network)?,
            shared_key,
        })
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn policy(&self) -> Policy {
        self.policy.clone()
    }

    pub fn shared_key(&self) -> SecretKey {
        self.shared_key
    }
}

impl From<&Vault> for ProtoVault {
    fn from(vault: &Vault) -> Self {
        ProtoVault {
            object: Some(ProtoVaultObject::V1(ProtoVaultV1 {
                descriptor: vault.as_descriptor().to_string(),
                network: vault.network().magic().to_bytes().to_vec(),
                shared_key: vault.shared_key.secret_bytes().to_vec(),
            })),
        }
    }
}

impl TryFrom<ProtoVault> for Vault {
    type Error = Error;
    fn try_from(vault: ProtoVault) -> Result<Self, Self::Error> {
        match vault.object {
            Some(obj) => match obj {
                ProtoVaultObject::V1(v) => {
                    let descriptor: Descriptor<String> = Descriptor::from_str(&v.descriptor)?;
                    let network: NetworkMagic = NetworkMagic::from_slice(&v.network)?;
                    let shared_key: SecretKey = SecretKey::from_slice(&v.shared_key)?;

                    Ok(Self {
                        version: Version::V1,
                        policy: Policy::new(descriptor, *network)?,
                        shared_key,
                    })
                }
            },
            None => Err(Error::NotFound(String::from("protobuf vault obj"))),
        }
    }
}

impl ProtocolEncoding for Vault {
    type Err = Error;

    fn pre_encoding(&self) -> (SchemaVersion, Vec<u8>) {
        let vault: ProtoVault = self.into();
        (SchemaVersion::ProtoBuf, vault.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoVault = ProtoVault::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for Vault {
    type Err = Error;
}

/// Build [`Vault`] invitation [`Event`]
pub fn build_invitation_event(vault: &Vault, receiver: XOnlyPublicKey) -> Result<Event, Error> {
    // Compose wrapper
    let wrapper: Wrapper = Wrapper::VaultInvite {
        vault: vault.clone(),
    };

    // Encrypt
    let keys = Keys::generate();
    let encrypted_content: String = wrapper.encrypt_with_keys(&keys).unwrap();

    // Compose and sign event
    Ok(EventBuilder::new(
        WRAPPER_KIND,
        encrypted_content,
        &[
            Tag::PubKey(receiver, None),
            Tag::Expiration(Timestamp::now() + WRAPPER_EXIPRATION),
        ],
    )
    .to_event(&keys)?)
}

/// Build [`Vault`] event (used to accept an invitation)
///
/// Must use **own** [`Keys`] (not random or shared key)!
pub fn build_event(keys: &Keys, vault: &Vault) -> Result<Event, Error> {
    // Encrypt
    let encrypted_content: String = vault.encrypt_with_keys(keys)?;

    // Compose and build event
    Ok(EventBuilder::new(VAULT_KIND_V2, encrypted_content, &[]).to_event(keys)?)
}
