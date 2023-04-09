// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use coinstr_core::bdk::database::MemoryDatabase;
use coinstr_core::bdk::Wallet;
use coinstr_core::nostr_sdk::{EventId, Result};
use coinstr_core::policy::Policy;
use coinstr_core::util::serde::{deserialize, serialize};
use sled::Tree;
use tokio::sync::Mutex;

const SHARED_KEYS: &str = "shared_keys";
const POLICIES: &str = "policies";

#[derive(Debug, Clone)]
pub struct Cache {
    pub shared_keys: Tree,
    pub policies: Tree,
    pub wallets: Arc<Mutex<HashMap<EventId, Wallet<MemoryDatabase>>>>,
}

impl Cache {
    pub fn new<P>(path: P) -> Self
    where
        P: AsRef<Path>,
    {
        let db = sled::open(path).expect("Impossible to open cache");
        Self {
            shared_keys: db.open_tree(SHARED_KEYS).expect("Impossible to open tree"),
            policies: db.open_tree(POLICIES).expect("Impossible to open tree"),
            wallets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn policy_exists(&self, policy_id: EventId) -> Result<bool> {
        Ok(self.policies.contains_key(serialize(policy_id)?)?)
    }

    pub fn get_policies(&self) -> Result<Vec<(EventId, Policy)>> {
        let mut policies = Vec::new();
        for res in self.policies.into_iter() {
            let (key, value) = res?;
            let event_id: EventId = deserialize(key.to_vec())?;
            let policy: Policy = deserialize(value.to_vec())?;
            policies.push((event_id, policy))
        }
        Ok(policies)
    }

    pub fn insert_policy(&self, policy_id: EventId, policy: Policy) -> Result<()> {
        let key = serialize(policy_id)?;
        let value = serialize(policy)?;
        self.policies.insert(key, value)?;
        log::info!("Saved policy {policy_id}");
        Ok(())
    }
}
