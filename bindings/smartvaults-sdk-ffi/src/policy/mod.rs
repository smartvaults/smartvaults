// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use nostr_ffi::{EventId, Timestamp};
use smartvaults_sdk::core::{policy, SelectableCondition};
use smartvaults_sdk::protocol::v1;
use smartvaults_sdk::protocol::v1::util::SerdeSer;
use smartvaults_sdk::types;
use uniffi::{Enum, Object, Record};

mod template;

pub use self::template::{
    AbsoluteLockTime, DecayingTime, Locktime, PolicyTemplate, PolicyTemplateType, RecoveryTemplate,
    RelativeLockTime,
};
use crate::error::Result;
use crate::{Balance, Signer};

#[derive(Clone, Object)]
pub struct Vault {
    inner: v1::Vault,
}

impl Deref for Vault {
    type Target = v1::Vault;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<v1::Vault> for Vault {
    fn from(inner: v1::Vault) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl Vault {
    pub fn name(&self) -> String {
        self.inner.name()
    }

    pub fn description(&self) -> String {
        self.inner.description()
    }

    pub fn descriptor(&self) -> String {
        self.inner.as_descriptor().to_string()
    }

    pub fn satisfiable_item(&self) -> Result<String> {
        Ok(self.inner.satisfiable_item()?.as_json())
    }

    pub fn has_timelock(&self) -> bool {
        self.inner.has_timelock()
    }

    pub fn selectable_conditions(&self) -> Result<Option<HashMap<String, Vec<String>>>> {
        Ok(self.inner.selectable_conditions()?.map(|list| {
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
    ) -> Result<Option<PolicyPathSelector>> {
        let res = self
            .inner
            .get_policy_path_from_signer(signer.as_ref().deref())?;
        Ok(res.map(|pp| pp.into()))
    }

    pub fn get_policy_paths_from_signers(&self, signers: Vec<Arc<Signer>>) -> Result<PolicyPath> {
        Ok(self
            .inner
            .get_policy_paths_from_signers(signers.into_iter().map(|s| s.as_ref().deref().clone()))?
            .into())
    }

    pub fn template_match(&self) -> Result<Option<PolicyTemplateType>> {
        Ok(self.inner.template_match()?.map(|t| t.into()))
    }
}

#[derive(Clone, Object)]
pub struct GetPolicy {
    inner: types::GetPolicy,
}

impl From<types::GetPolicy> for GetPolicy {
    fn from(inner: types::GetPolicy) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl GetPolicy {
    pub fn policy_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.policy_id.into())
    }

    pub fn vault(&self) -> Arc<Vault> {
        Arc::new(self.inner.vault.clone().into())
    }

    pub fn balance(&self) -> Arc<Balance> {
        Arc::new(self.inner.balance.clone().into())
    }

    pub fn last_sync(&self) -> Arc<Timestamp> {
        Arc::new(self.inner.last_sync.into())
    }
}

#[derive(Enum)]
pub enum PolicyPath {
    Single { pp: PolicyPathSelector },
    Multiple { list: Vec<PolicyPathSigner> },
    None,
}

impl From<policy::PolicyPath> for PolicyPath {
    fn from(value: policy::PolicyPath) -> Self {
        match value {
            policy::PolicyPath::Single(pp) => Self::Single { pp: pp.into() },
            policy::PolicyPath::Multiple(list) => Self::Multiple {
                list: list
                    .into_iter()
                    .map(|(s, pp)| PolicyPathSigner {
                        signer: Arc::new(s.into()),
                        policy_path: pp.map(|pp| pp.into()),
                    })
                    .collect(),
            },
            policy::PolicyPath::None => Self::None,
        }
    }
}

#[derive(Record)]
pub struct PolicyPathSigner {
    pub signer: Arc<Signer>,
    pub policy_path: Option<PolicyPathSelector>,
}

#[derive(Enum)]
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
