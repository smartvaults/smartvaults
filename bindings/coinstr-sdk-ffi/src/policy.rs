// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::ops::Deref;
use std::sync::Arc;

use coinstr_sdk::core::policy;
use coinstr_sdk::db::model;
use nostr_ffi::Timestamp;

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
    pub fn policy_id(&self) -> String {
        self.inner.policy_id.to_string()
    }

    pub fn policy(&self) -> Arc<Policy> {
        Arc::new(self.inner.policy.clone().into())
    }

    pub fn last_sync(&self) -> Option<Arc<Timestamp>> {
        self.inner.last_sync.map(|t| Arc::new(t.into()))
    }
}
