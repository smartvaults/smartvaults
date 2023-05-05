// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;
use std::sync::Arc;

use coinstr_core::cache;
use coinstr_core::nostr_sdk::{block_on, EventId};

use crate::error::Result;
use crate::policy::Policy;

pub struct Cache {
    inner: cache::Cache,
}

impl From<cache::Cache> for Cache {
    fn from(inner: cache::Cache) -> Self {
        Self { inner }
    }
}

impl Cache {
    pub fn block_height(&self) -> u32 {
        self.inner.block_height()
    }

    pub fn policies(&self) -> HashMap<String, Arc<Policy>> {
        block_on(async move {
            let policies = self.inner.policies().await;
            policies
                .into_iter()
                .map(|(policy_id, policy)| (policy_id.to_string(), Arc::new(policy.into())))
                .collect()
        })
    }

    pub fn get_policy_by_id(&self, policy_id: String) -> Result<Option<Arc<Policy>>> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            let policy = self.inner.get_policy_by_id(policy_id).await;
            Ok(policy.map(|p| Arc::new(p.into())))
        })
    }
}
