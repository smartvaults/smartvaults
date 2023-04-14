// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use coinstr_core::bdk::blockchain::Blockchain;
use coinstr_core::bdk::database::MemoryDatabase;
use coinstr_core::bdk::{Balance, SyncOptions, TransactionDetails, Wallet};
use coinstr_core::bitcoin::Network;
use coinstr_core::nostr_sdk::{EventId, Result};
use coinstr_core::policy::Policy;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::util::serde::{deserialize, serialize};
use sled::Tree;
use tokio::sync::Mutex;

const SHARED_KEYS: &str = "shared_keys";
const POLICIES: &str = "policies";
const PROPOSALS: &str = "proposals";

#[derive(Debug, Clone)]
pub struct Cache {
    pub shared_keys: Tree,
    pub policies: Tree,
    pub proposals: Tree,
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
            proposals: db.open_tree(PROPOSALS).expect("Impossible to open tree"),
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

    pub fn proposal_exists(&self, proposal_id: EventId) -> Result<bool> {
        Ok(self.proposals.contains_key(serialize(proposal_id)?)?)
    }

    pub fn get_proposals(&self) -> Result<Vec<(EventId, SpendingProposal)>> {
        let mut proposals = Vec::new();
        for res in self.proposals.into_iter() {
            let (key, value) = res?;
            let event_id: EventId = deserialize(key.to_vec())?;
            let proposal: SpendingProposal = deserialize(value.to_vec())?;
            proposals.push((event_id, proposal))
        }
        Ok(proposals)
    }

    pub fn insert_proposal(&self, proposal_id: EventId, proposal: SpendingProposal) -> Result<()> {
        let key = serialize(proposal_id)?;
        let value = serialize(proposal)?;
        self.policies.insert(key, value)?;
        log::info!("Saved spending proposal {proposal_id}");
        Ok(())
    }

    pub async fn load_wallets(&self, network: Network) -> Result<()> {
        let mut wallets = self.wallets.lock().await;
        for (policy_id, policy) in self.get_policies()?.into_iter() {
            if let Entry::Vacant(e) = wallets.entry(policy_id) {
                let db = MemoryDatabase::new();
                let wallet = Wallet::new(&policy.descriptor.to_string(), None, network, db)?;
                e.insert(wallet);
            }
        }
        Ok(())
    }

    pub async fn sync_wallets<B>(&self, blockchain: &B) -> Result<()>
    where
        B: Blockchain,
    {
        let wallets = self.wallets.lock().await;
        for (policy_id, wallet) in wallets.iter() {
            log::info!("Syncing policy {policy_id}");
            wallet.sync(blockchain, SyncOptions::default())?;
        }
        Ok(())
    }

    pub async fn get_balance(&self, policy_id: EventId) -> Option<Balance> {
        let wallets = self.wallets.lock().await;
        let wallet = wallets.get(&policy_id)?;
        wallet.get_balance().ok()
    }

    pub async fn get_transactions(&self, policy_id: EventId) -> Option<Vec<TransactionDetails>> {
        let wallets = self.wallets.lock().await;
        let wallet = wallets.get(&policy_id)?;
        wallet.list_transactions(false).ok()
    }
}
