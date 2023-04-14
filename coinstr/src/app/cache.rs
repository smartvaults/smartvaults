// Copyright (c) 2022-2023 Yuki Kishimoto
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
pub struct CacheWallet {
    wallet: Wallet<MemoryDatabase>,
    last_sync: Timestamp,
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub policies: Arc<Mutex<HashMap<EventId, Policy>>>,
    pub proposals: Arc<Mutex<HashMap<EventId, (EventId, SpendingProposal)>>>,
    pub wallets: Arc<Mutex<HashMap<EventId, CacheWallet>>>,
}

impl Cache {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(Mutex::new(HashMap::new())),
            proposals: Arc::new(Mutex::new(HashMap::new())),
            wallets: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn policy_exists(&self, policy_id: EventId) -> bool {
        let policies = self.policies.lock().await;
        policies.contains_key(&policy_id)
    }

    pub async fn policies(&self) -> HashMap<EventId, Policy> {
        let policies = self.policies.lock().await;
        policies.clone()
    }

    pub async fn cache_policy(
        &self,
        policy_id: EventId,
        policy: Policy,
        network: Network,
    ) -> Result<()> {
        let mut policies = self.policies.lock().await;
        if let Entry::Vacant(e) = policies.entry(policy_id) {
            let descriptor: &str = &policy.descriptor.to_string();

            // Cache policy
            e.insert(policy);
            log::info!("Cached policy {policy_id}");

            // Load wallet
            let mut wallets = self.wallets.lock().await;
            if let Entry::Vacant(e) = wallets.entry(policy_id) {
                let db = MemoryDatabase::new();
                let wallet = Wallet::new(descriptor, None, network, db)?;
                e.insert(CacheWallet {
                    wallet,
                    last_sync: Timestamp::from(0),
                });
            }
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
        let mut wallets = self.wallets.lock().await;
        for (policy_id, cache) in wallets.iter_mut() {
            if cache.last_sync.add(WALLET_SYNC_INTERVAL) <= Timestamp::now() {
                log::info!("Syncing policy {policy_id}");
                cache.wallet.sync(blockchain, SyncOptions::default())?;
                cache.last_sync = Timestamp::now();
            }
        }
        Ok(())
    }

    pub async fn get_balance(&self, policy_id: EventId) -> Option<Balance> {
        let wallets = self.wallets.lock().await;
        let cache = wallets.get(&policy_id)?;
        cache.wallet.get_balance().ok()
    }

    pub async fn get_transactions(&self, policy_id: EventId) -> Option<Vec<TransactionDetails>> {
        let wallets = self.wallets.lock().await;
        let cache = wallets.get(&policy_id)?;
        cache.wallet.list_transactions(false).ok()
    }
}
