// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;
use std::str::FromStr;

use prost::Message;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::policy::{self, Policy};
use smartvaults_core::PolicyTemplate;
use thiserror::Error;

mod proto;

use super::schema::{self, Schema, Version as SchemaVersion};
use super::NetworkMagic;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Policy(#[from] policy::Error),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    #[default]
    V1 = 0x01,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultMetadata {
    name: String,
    description: String,
}

impl VaultMetadata {
    pub fn new<S>(name: S, description: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vault {
    version: Version,
    policy: Policy,
    metadata: VaultMetadata,
}

impl Deref for Vault {
    type Target = Policy;
    fn deref(&self) -> &Self::Target {
        &self.policy
    }
}

impl Vault {
    pub fn new<S, P>(
        name: S,
        description: S,
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
            metadata: VaultMetadata::new(name, description),
        })
    }

    pub fn from_template<S>(
        name: S,
        description: S,
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
            metadata: VaultMetadata::new(name, description),
        })
    }

    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    pub fn description(&self) -> &str {
        &self.metadata.description
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn policy(&self) -> Policy {
        self.policy.clone()
    }

    pub fn decode<T>(payload: T) -> Result<Self, Error>
    where
        T: AsRef<[u8]>,
    {
        let Schema { version, data } = schema::decode(payload.as_ref()).unwrap();
        match version {
            SchemaVersion::ProtoBuf => {
                let vault = proto::Vault::decode(data).unwrap();
                match vault.object {
                    Some(obj) => match obj {
                        proto::vault::Object::V1(v) => {
                            let descriptor = Descriptor::from_str(&v.descriptor).unwrap();
                            let network = NetworkMagic::from_slice(&v.network).unwrap();
                            Ok(Self {
                                version: Version::V1,
                                policy: Policy::new(descriptor, *network)?,
                                metadata: VaultMetadata::default(), // TODO: decode metadata
                            })
                        }
                    },
                    None => panic!("Vault obj not found"),
                }
            }
        }
    }

    pub fn encode(&self) -> Vec<u8> {
        let v1 = proto::VaultV1 {
            descriptor: self.as_descriptor().to_string(),
            network: self.network().magic().to_bytes().to_vec(),
        };
        let mut vault = proto::Vault::default();
        vault.object = Some(proto::vault::Object::V1(v1));
        let ser = vault.encode_to_vec();
        schema::encode(ser, SchemaVersion::ProtoBuf)
    }
}
