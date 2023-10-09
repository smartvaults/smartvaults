// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use nostr_sdk_ffi::{EventId, Timestamp};
use smartvaults_sdk::core::{policy, PolicyTemplateType, SelectableCondition};
use smartvaults_sdk::protocol::v1::util::SerdeSer;
use smartvaults_sdk::types;

mod template;

pub use self::template::{AbsoluteLockTime, PolicyTemplate, RecoveryTemplate, RelativeLockTime};
use crate::error::Result;
use crate::{Balance, Network, Signer};

#[derive(Clone)]
pub struct Policy {
    inner: policy::Policy,
}

impl Deref for Policy {
    type Target = policy::Policy;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<policy::Policy> for Policy {
    fn from(inner: policy::Policy) -> Self {
        Self { inner }
    }
}

impl Policy {
    pub fn name(&self) -> String {
        self.inner.name.clone()
    }

    pub fn description(&self) -> String {
        self.inner.description.clone()
    }

    pub fn descriptor(&self) -> String {
        self.inner.descriptor.to_string()
    }

    pub fn satisfiable_item(&self, network: Network) -> Result<String> {
        Ok(self.inner.satisfiable_item(network.into())?.as_json())
    }

    pub fn has_timelock(&self) -> bool {
        self.inner.has_timelock()
    }

    pub fn selectable_conditions(
        &self,
        network: Network,
    ) -> Result<Option<HashMap<String, Vec<String>>>> {
        Ok(self
            .inner
            .selectable_conditions(network.into())?
            .map(|list| {
                list.into_iter()
                    .map(
                        |SelectableCondition {
                             path, sub_paths, ..
                         }| (path, sub_paths),
                    )
                    .collect()
            }))
    }

    pub fn search_used_signers(&self, signers: Vec<Arc<Signer>>) -> Result<Vec<Arc<Signer>>> {
        Ok(self
            .inner
            .search_used_signers(signers.into_iter().map(|s| s.as_ref().deref().clone()))?
            .into_iter()
            .map(|s| Arc::new(s.into()))
            .collect())
    }

    pub fn get_policy_path_from_signer(
        &self,
        signer: Arc<Signer>,
        network: Network,
    ) -> Result<Option<PolicyPathSelector>> {
        let res = self
            .inner
            .get_policy_path_from_signer(signer.as_ref().deref(), network.into())?;
        Ok(res.map(|pp| pp.into()))
    }

    pub fn get_policy_paths_from_signers(
        &self,
        signers: Vec<Arc<Signer>>,
        network: Network,
    ) -> Result<PolicyPath> {
        Ok(self
            .inner
            .get_policy_paths_from_signers(
                signers.into_iter().map(|s| s.as_ref().deref().clone()),
                network.into(),
            )?
            .into())
    }

    pub fn template_match(&self, network: Network) -> Result<Option<PolicyTemplateType>> {
        Ok(self.inner.template_match(network.into())?)
    }
}

#[derive(Debug, Clone)]
pub struct GetPolicy {
    inner: types::GetPolicy,
}

impl From<types::GetPolicy> for GetPolicy {
    fn from(inner: types::GetPolicy) -> Self {
        Self { inner }
    }
}

impl GetPolicy {
    pub fn policy_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.policy_id.into())
    }

    pub fn policy(&self) -> Arc<Policy> {
        Arc::new(self.inner.policy.clone().into())
    }

    pub fn balance(&self) -> Arc<Balance> {
        Arc::new(self.inner.balance.clone().into())
    }

    pub fn last_sync(&self) -> Option<Arc<Timestamp>> {
        self.inner.last_sync.map(|t| Arc::new(t.into()))
    }
}

pub enum PolicyPath {
    Single {
        pp: PolicyPathSelector,
    },
    Multiple {
        map: HashMap<Arc<Signer>, Option<PolicyPathSelector>>,
    },
    None,
}

impl From<policy::PolicyPath> for PolicyPath {
    fn from(value: policy::PolicyPath) -> Self {
        match value {
            policy::PolicyPath::Single(pp) => Self::Single { pp: pp.into() },
            policy::PolicyPath::Multiple(map) => Self::Multiple {
                map: map
                    .into_iter()
                    .map(|(s, pp)| (Arc::new(s.into()), pp.map(|pp| pp.into())))
                    .collect(),
            },
            policy::PolicyPath::None => Self::None,
        }
    }
}

pub enum PolicyPathSelector {
    Complete {
        path: HashMap<String, Vec<u64>>,
    },
    Partial {
        selected_path: HashMap<String, Vec<u64>>,
        missing_to_select: HashMap<String, Vec<String>>,
    },
}

impl From<policy::PolicyPathSelector> for PolicyPathSelector {
    fn from(pps: policy::PolicyPathSelector) -> Self {
        match pps {
            policy::PolicyPathSelector::Complete { path } => Self::Complete {
                path: path
                    .into_iter()
                    .map(|(k, v)| (k, v.into_iter().map(|n| n as u64).collect()))
                    .collect(),
            },
            policy::PolicyPathSelector::Partial {
                selected_path,
                missing_to_select,
            } => Self::Partial {
                selected_path: selected_path
                    .into_iter()
                    .map(|(k, v)| (k, v.into_iter().map(|n| n as u64).collect()))
                    .collect(),
                missing_to_select: missing_to_select.into_iter().collect(),
            },
        }
    }
}
