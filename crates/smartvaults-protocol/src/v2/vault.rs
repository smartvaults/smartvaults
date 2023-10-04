// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;

use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::policy::{self, Policy};
use smartvaults_core::PolicyTemplate;
use thiserror::Error;

use super::NetworkMagic;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Policy(#[from] policy::Error),
}

/// Object used for de/serialization to easily handle the versions
#[derive(Serialize, Deserialize)]
#[serde(tag = "version")]
enum VaultObject {
    V1 {
        /// Name
        name: String,
        /// Description
        description: String,
        /// Descriptor
        descriptor: Descriptor<String>,
        /// Network magic
        network: NetworkMagic,
    },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    #[default]
    V1 = 0x01,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Vault {
    name: String,
    description: String,
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
    pub fn new<N, D, P>(
        name: N,
        description: D,
        descriptor: P,
        network: Network,
    ) -> Result<Self, policy::Error>
    where
        N: Into<String>,
        D: Into<String>,
        P: Into<String>,
    {
        let policy: Policy = Policy::from_desc_or_miniscript(descriptor, network)?;
        Ok(Self {
            name: name.into(),
            description: description.into(),
            version: Version::default(),
            policy,
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
            name: name.into(),
            description: description.into(),
            version: Version::default(),
            policy,
        })
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn version(&self) -> Version {
        self.version
    }

    pub fn policy(&self) -> Policy {
        self.policy.clone()
    }
}

impl TryFrom<VaultObject> for Vault {
    type Error = Error;
    fn try_from(obj: VaultObject) -> Result<Self, Self::Error> {
        match obj {
            VaultObject::V1 {
                name,
                description,
                descriptor,
                network,
            } => Ok(Self {
                name,
                description,
                version: Version::V1,
                policy: Policy::new(descriptor, *network)?,
            }),
        }
    }
}

impl From<Vault> for VaultObject {
    fn from(vault: Vault) -> Self {
        match vault.version {
            Version::V1 => VaultObject::V1 {
                descriptor: vault.descriptor(),
                network: vault.network().into(),
                name: vault.name,
                description: vault.description,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_vault_serialization() {
        let vault_obj = VaultObject::V1 {
            name: String::from("Name"), 
            description: String::from("Desc"), 
            descriptor: Descriptor::from_str("tr([5cb492a5/86'/1'/784923']tpubDD56LAR1MR7X5EeZYMpvivk2Lh3HMo4vdDNQ8jAv4oBjLPEddQwxaxNypvrHbMk2qTxAj44YLzqHrzwy5LDNmVyYZBesm6aShhmhYrA8veT/0/*,{pk([76fdbca2/86'/1'/784923']tpubDCDepsNyAPWySAgXx1Por6sHpSWzxsTB9XJp5erEN7NumgdZMhhmycJGMQ1cHZwx66KyZr6psjttDDQ7mV4uJGV2DvB9Mri1nTVmpquvTDR/0/*),pk([3b8ae29b/86'/1'/784923']tpubDDpkQsJQTpHi2bH5Cg7L1pThUxeEStcn9ZsQ53XHkW8Fs81h71XobqpwYf2Jb8ECmW1mUUJxQhZstmwFUg5wQ6EVzH5HmF3cpHcyxjvF1Ep/0/*)})#yxpuntg3").unwrap(),
            network: Network::Testnet.into(),
        };
        let ser: String = serde_json::to_string(&vault_obj).unwrap();
        assert_eq!(
            &ser,
            r##"{"version":"V1","name":"Name","description":"Desc","descriptor":"tr([5cb492a5/86'/1'/784923']tpubDD56LAR1MR7X5EeZYMpvivk2Lh3HMo4vdDNQ8jAv4oBjLPEddQwxaxNypvrHbMk2qTxAj44YLzqHrzwy5LDNmVyYZBesm6aShhmhYrA8veT/0/*,{pk([76fdbca2/86'/1'/784923']tpubDCDepsNyAPWySAgXx1Por6sHpSWzxsTB9XJp5erEN7NumgdZMhhmycJGMQ1cHZwx66KyZr6psjttDDQ7mV4uJGV2DvB9Mri1nTVmpquvTDR/0/*),pk([3b8ae29b/86'/1'/784923']tpubDDpkQsJQTpHi2bH5Cg7L1pThUxeEStcn9ZsQ53XHkW8Fs81h71XobqpwYf2Jb8ECmW1mUUJxQhZstmwFUg5wQ6EVzH5HmF3cpHcyxjvF1Ep/0/*)})#yxpuntg3","network":[11,17,9,7]}"##
        );
    }
}
