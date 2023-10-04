// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;

use serde::de::Error as DeserializerError;
use serde::{Deserialize, Deserializer, Serialize};
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::{policy, Policy, PolicyTemplate};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct Vault {
    pub name: String,
    pub description: String,
    #[serde(skip)]
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
            policy,
        })
    }

    pub fn policy(&self) -> Policy {
        self.policy.clone()
    }
}

#[derive(Deserialize)]
struct VaultIntermediate {
    name: String,
    description: String,
    descriptor: Descriptor<String>,
}

impl<'de> Deserialize<'de> for Vault {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let intermediate: VaultIntermediate = VaultIntermediate::deserialize(deserializer)?;
        let network = Network::Testnet; // TODO: search network
        Ok(Self {
            name: intermediate.name,
            description: intermediate.description,
            policy: Policy::new(intermediate.descriptor.clone(), network)
                .map_err(DeserializerError::custom)?,
        })
    }
}
