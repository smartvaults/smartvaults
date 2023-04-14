// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;

use coinstr_core::bdk::blockchain::Blockchain;
use coinstr_core::bdk::database::MemoryDatabase;
use coinstr_core::bdk::{Balance, SyncOptions, TransactionDetails, Wallet};
use coinstr_core::bitcoin::Network;
use coinstr_core::nostr_sdk::{EventId, Result, Timestamp};
use coinstr_core::policy::Policy;
use coinstr_core::proposal::SpendingProposal;
use tokio::sync::Mutex;

const WALLET_SYNC_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug)]
pub struct PolicyWallet {
    policy: Policy,
    wallet: Wallet<MemoryDatabase>,
    last_sync: Timestamp,
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub policies: Arc<Mutex<HashMap<EventId, PolicyWallet>>>,
    pub proposals: Arc<Mutex<HashMap<EventId, (EventId, SpendingProposal)>>>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(Mutex::new(HashMap::new())),
            proposals: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn policy_exists(&self, policy_id: EventId) -> bool {
        let policies = self.policies.lock().await;
        policies.contains_key(&policy_id)
    }

    pub async fn policies(&self) -> HashMap<EventId, Policy> {
        let policies = self.policies.lock().await;
        policies
            .iter()
            .map(|(policy_id, w)| (*policy_id, w.policy.clone()))
            .collect()
    }

    pub async fn cache_policy(
        &self,
        policy_id: EventId,
        policy: Policy,
        network: Network,
    ) -> Result<()> {
        let mut policies = self.policies.lock().await;
        if let Entry::Vacant(e) = policies.entry(policy_id) {
            // Cache policy
            let db = MemoryDatabase::new();
            let wallet = Wallet::new(&policy.descriptor.to_string(), None, network, db)?;
            e.insert(PolicyWallet {
                policy,
                wallet,
                last_sync: Timestamp::from(0),
            });
            log::info!("Cached policy {policy_id}");
        }
        Ok(())
    }

    pub async fn proposal_exists(&self, proposal_id: EventId) -> bool {
        let proposals = self.proposals.lock().await;
        proposals.contains_key(&proposal_id)
    }

    pub async fn proposals(&self) -> HashMap<EventId, (EventId, SpendingProposal)> {
        let proposals = self.proposals.lock().await;
        proposals.clone()
    }

    pub async fn cache_proposal(
        &self,
        proposal_id: EventId,
        policy_id: EventId,
        proposal: SpendingProposal,
    ) {
        let mut proposals = self.proposals.lock().await;
        if let Entry::Vacant(e) = proposals.entry(proposal_id) {
            e.insert((policy_id, proposal));
            log::info!("Cached spending proposal {proposal_id}");
        }
    }

    pub async fn sync_wallets<B>(&self, blockchain: &B) -> Result<()>
    where
        B: Blockchain,
    {
        let mut policies = self.policies.lock().await;
        for (policy_id, pw) in policies.iter_mut() {
            if pw.last_sync.add(WALLET_SYNC_INTERVAL) <= Timestamp::now() {
                log::info!("Syncing policy {policy_id}");
                pw.wallet.sync(blockchain, SyncOptions::default())?;
                pw.last_sync = Timestamp::now();
            }
        }
        Ok(())
    }

    pub async fn get_balance(&self, policy_id: EventId) -> Option<Balance> {
        let policies = self.policies.lock().await;
        let pw = policies.get(&policy_id)?;
        pw.wallet.get_balance().ok()
    }

    pub async fn get_transactions(&self, policy_id: EventId) -> Option<Vec<TransactionDetails>> {
        let policies = self.policies.lock().await;
        let pw = policies.get(&policy_id)?;
        pw.wallet.list_transactions(false).ok()
    }

    pub async fn get_total_balance(&self) -> Result<Balance> {
        let policies = self.policies.lock().await;
        let mut total_balance = Balance::default();
        let mut already_seen = Vec::new();
        for (_, pw) in policies.iter() {
            if !already_seen.contains(&&pw.policy.descriptor) {
                let balance = pw.wallet.get_balance()?;
                total_balance = total_balance.add(balance);
                already_seen.push(&pw.policy.descriptor);
            }
        }
        Ok(total_balance)
    }

    pub async fn get_all_transactions(&self) -> Result<Vec<TransactionDetails>> {
        let policies = self.policies.lock().await;
        let mut transactions = Vec::new();
        let mut already_seen = Vec::new();
        for (_, pw) in policies.iter() {
            if !already_seen.contains(&&pw.policy.descriptor) {
                let mut list = pw.wallet.list_transactions(false)?;
                transactions.append(&mut list);
                already_seen.push(&pw.policy.descriptor);
            }
        }
        Ok(transactions)
    }
}
