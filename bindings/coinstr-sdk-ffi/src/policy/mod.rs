// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;
use std::ops::Deref;
use std::sync::Arc;

use coinstr_sdk::core::util::serde::SerdeSer;
use coinstr_sdk::core::{policy, PolicyTemplateType};
use coinstr_sdk::db::model;
use nostr_ffi::{EventId, Timestamp};

mod template;

pub use self::template::{AbsoluteLockTime, PolicyTemplate, RecoveryTemplate, Sequence};
use crate::error::Result;
use crate::Network;

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
            .map(|list| list.into_iter().collect()))
    }

    pub fn template_match(&self, network: Network) -> Result<Option<PolicyTemplateType>> {
        Ok(self.inner.template_match(network.into())?)
    }
}

#[derive(Debug, Clone)]
pub struct GetPolicy {
    inner: model::GetPolicy,
}

impl From<model::GetPolicy> for GetPolicy {
    fn from(inner: model::GetPolicy) -> Self {
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

    pub fn last_sync(&self) -> Option<Arc<Timestamp>> {
        self.inner.last_sync.map(|t| Arc::new(t.into()))
    }
}