// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::ops::Add;
use std::sync::Arc;
use std::time::Duration;

use coinstr_core::bdk::blockchain::Blockchain;
use coinstr_core::bdk::database::MemoryDatabase;
use coinstr_core::bdk::{Balance, SyncOptions, TransactionDetails, Wallet};
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Network, Txid, XOnlyPublicKey};
use coinstr_core::nostr_sdk::{EventId, Result, Timestamp};
use coinstr_core::policy::Policy;
use coinstr_core::proposal::SpendingProposal;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

const WALLET_SYNC_INTERVAL: Duration = Duration::from_secs(60);

type ApprovedProposals =
    BTreeMap<EventId, BTreeMap<XOnlyPublicKey, (EventId, PartiallySignedTransaction, Timestamp)>>;

#[derive(Debug)]
pub struct PolicyWallet {
    policy: Policy,
    wallet: Wallet<MemoryDatabase>,
    last_sync: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub policies: Arc<Mutex<BTreeMap<EventId, PolicyWallet>>>,
    pub proposals: Arc<Mutex<BTreeMap<EventId, (EventId, SpendingProposal)>>>,
    pub approved_proposals: Arc<Mutex<ApprovedProposals>>,
}

impl Default for Cache {
    fn default() -> Self {
        Self::new()
    }
}

impl Cache {
    pub fn new() -> Self {
        Self {
            policies: Arc::new(Mutex::new(BTreeMap::new())),
            proposals: Arc::new(Mutex::new(BTreeMap::new())),
            approved_proposals: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub async fn policy_exists(&self, policy_id: EventId) -> bool {
        let policies = self.policies.lock().await;
        policies.contains_key(&policy_id)
    }

    pub async fn policies(&self) -> BTreeMap<EventId, Policy> {
        let policies = self.policies.lock().await;
        policies
            .iter()
            .map(|(policy_id, w)| (*policy_id, w.policy.clone()))
            .collect()
    }

    pub async fn policies_with_balance(
        &self,
    ) -> BTreeMap<EventId, (Policy, Option<Balance>, bool)> {
        let policies = self.policies.lock().await;
        let mut new_policies = BTreeMap::new();
        for (policy_id, pw) in policies.iter() {
            new_policies.insert(
                *policy_id,
                (
                    pw.policy.clone(),
                    pw.wallet.get_balance().ok(),
                    pw.last_sync.is_some(),
                ),
            );
        }
        new_policies
    }

    pub async fn policy(
        &self,
        policy_id: EventId,
    ) -> Option<(
        Policy,
        Option<Balance>,
        Option<Vec<TransactionDetails>>,
        Option<Timestamp>,
    )> {
        let policies = self.policies.lock().await;
        let pw = policies.get(&policy_id)?;
        let balance = pw.wallet.get_balance().ok();
        let list = pw.wallet.list_transactions(false).ok();
        Some((pw.policy.clone(), balance, list, pw.last_sync))
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
                last_sync: None,
            });
            log::info!("Cached policy {policy_id}");
        }
        Ok(())
    }

    pub async fn proposal_exists(&self, proposal_id: EventId) -> bool {
        let proposals = self.proposals.lock().await;
        proposals.contains_key(&proposal_id)
    }

    pub async fn proposals(&self) -> BTreeMap<EventId, (EventId, SpendingProposal)> {
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

    pub async fn uncache_proposal(&self, proposal_id: EventId) {
        let mut proposals = self.proposals.lock().await;
        proposals.remove(&proposal_id);
        let mut approved_proposals = self.approved_proposals.lock().await;
        approved_proposals.remove(&proposal_id);
        log::info!("Proposal {proposal_id} removed");
    }

    pub async fn approved_proposals(&self) -> ApprovedProposals {
        let approved_proposals = self.approved_proposals.lock().await;
        approved_proposals.clone()
    }

    pub async fn signed_psbts_by_proposal_id(
        &self,
        proposal_id: EventId,
    ) -> Option<BTreeMap<XOnlyPublicKey, (EventId, PartiallySignedTransaction, Timestamp)>> {
        let approved_proposals = self.approved_proposals.lock().await;
        approved_proposals.get(&proposal_id).cloned()
    }

    pub async fn cache_approved_proposal(
        &self,
        proposal_id: EventId,
        author: XOnlyPublicKey,
        approved_proposal_id: EventId,
        psbt: PartiallySignedTransaction,
        timestamp: Timestamp,
    ) {
        let mut approved_proposals = self.approved_proposals.lock().await;
        approved_proposals
            .entry(proposal_id)
            .and_modify(|map| {
                if let Entry::Vacant(e) = map.entry(author) {
                    e.insert((approved_proposal_id, psbt.clone(), timestamp));
                    log::info!(
                        "Cached approved proposal {proposal_id} for pubkey {author} (append)"
                    );
                }
            })
            .or_insert_with(|| {
                log::info!("Cached approved proposal {proposal_id} for pubkey {author}");
                [(author, (approved_proposal_id, psbt.clone(), timestamp))].into()
            });
    }

    pub async fn sync_wallets<B>(
        &self,
        blockchain: &B,
        sender: Option<&UnboundedSender<()>>,
        force: bool,
    ) -> Result<()>
    where
        B: Blockchain,
    {
        let mut policies = self.policies.lock().await;
        for (policy_id, pw) in policies.iter_mut() {
            let last_sync = pw.last_sync.unwrap_or_else(|| Timestamp::from(0));
            if force || last_sync.add(WALLET_SYNC_INTERVAL) <= Timestamp::now() {
                log::info!("Syncing policy {policy_id}");
                pw.wallet.sync(blockchain, SyncOptions::default())?;
                pw.last_sync = Some(Timestamp::now());
                if let Some(sender) = sender {
                    sender.send(())?;
                }
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

    pub async fn get_total_balance(&self) -> Result<(Balance, bool)> {
        let policies = self.policies.lock().await;
        let mut synced = true;
        let mut total_balance = Balance::default();
        let mut already_seen = Vec::new();
        for (_, pw) in policies.iter() {
            if !already_seen.contains(&&pw.policy.descriptor) {
                if pw.last_sync.is_none() {
                    synced = false;
                    break;
                }
                let balance = pw.wallet.get_balance()?;
                total_balance = total_balance.add(balance);
                already_seen.push(&pw.policy.descriptor);
            }
        }
        Ok((total_balance, synced))
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

    pub async fn get_tx(&self, txid: Txid) -> Option<TransactionDetails> {
        let policies = self.policies.lock().await;
        let mut already_seen = Vec::new();
        for (_, pw) in policies.iter() {
            if !already_seen.contains(&&pw.policy.descriptor) {
                let txs = pw.wallet.list_transactions(true).ok()?;
                for tx in txs.into_iter() {
                    if tx.txid == txid {
                        return Some(tx);
                    }
                }
                already_seen.push(&pw.policy.descriptor);
            }
        }
        None
    }
}
