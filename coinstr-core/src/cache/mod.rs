// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::btree_map::Entry;
use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::{BTreeMap, HashMap};
use std::ops::{Add, Sub};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_recursion::async_recursion;
use bdk::blockchain::Blockchain;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client as ElectrumClient;
use bdk::wallet::AddressIndex;
use bdk::{Balance, SyncOptions, TransactionDetails, Wallet};
use futures_util::future::{AbortHandle, Abortable};
use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::{Address, Network, Txid, XOnlyPublicKey};
use nostr_sdk::event::tag::TagKind;
use nostr_sdk::{Event, EventId, Filter, Keys, RelayPoolNotification, Result, Tag, Timestamp};
use tokio::sync::mpsc::error::SendError;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::constants::{
    APPROVED_PROPOSAL_EXPIRATION, APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, POLICY_KIND,
    PROPOSAL_KIND,
};
use crate::policy::Policy;
use crate::proposal::{ApprovedProposal, CompletedProposal, Proposal};
use crate::Encryption;
use crate::{util, CoinstrClient};

const BLOCK_HEIGHT_SYNC_INTERVAL: Duration = Duration::from_secs(60);
const WALLET_SYNC_INTERVAL: Duration = Duration::from_secs(60);

pub type Transactions = Vec<(TransactionDetails, Option<String>)>;
type ApprovedProposals =
    BTreeMap<EventId, BTreeMap<XOnlyPublicKey, (EventId, PartiallySignedTransaction, Timestamp)>>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error("channel send error")]
    SendError(#[from] SendError<()>),
}

#[derive(Debug, Clone, Default)]
pub struct BlockHeight {
    height: Arc<AtomicU32>,
    last_sync: Arc<Mutex<Option<Timestamp>>>,
}

impl BlockHeight {
    pub fn block_height(&self) -> u32 {
        self.height.load(Ordering::SeqCst)
    }

    pub fn set_block_height(&self, block_height: u32) {
        let _ = self
            .height
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(block_height));
    }

    pub async fn is_synced(&self) -> bool {
        let last_sync = self.last_sync.lock().await;
        let last_sync: Timestamp = last_sync.unwrap_or_else(|| Timestamp::from(0));
        last_sync.add(BLOCK_HEIGHT_SYNC_INTERVAL) > Timestamp::now()
    }

    pub async fn just_synced(&self) {
        let mut last_sync = self.last_sync.lock().await;
        *last_sync = Some(Timestamp::now());
    }
}

#[derive(Debug)]
pub struct PolicyWallet {
    policy: Policy,
    wallet: Wallet<MemoryDatabase>,
    last_sync: Option<Timestamp>,
}

#[derive(Debug, Clone, Default)]
pub struct Cache {
    pub block_height: BlockHeight,
    shared_keys: Arc<Mutex<HashMap<EventId, Keys>>>,
    pub policies: Arc<Mutex<BTreeMap<EventId, PolicyWallet>>>,
    pub proposals: Arc<Mutex<BTreeMap<EventId, (EventId, Proposal)>>>,
    pub approved_proposals: Arc<Mutex<ApprovedProposals>>,
    pub completed_proposals: Arc<Mutex<BTreeMap<EventId, (EventId, CompletedProposal)>>>,
}

impl Cache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn block_height(&self) -> u32 {
        self.block_height.block_height()
    }

    pub fn cache_block_height(&self, block_height: u32) {
        self.block_height.set_block_height(block_height);
    }

    pub async fn shared_key_by_policy_id(&self, policy_id: EventId) -> Option<Keys> {
        let shared_keys = self.shared_keys.lock().await;
        shared_keys.get(&policy_id).cloned()
    }

    pub async fn cache_shared_key(&self, policy_id: EventId, keys: Keys) {
        let mut shared_keys = self.shared_keys.lock().await;
        shared_keys.insert(policy_id, keys);
    }

    pub async fn cache_shared_keys(&self, map: HashMap<EventId, Keys>) {
        let mut shared_keys = self.shared_keys.lock().await;
        for (policy_id, keys) in map.into_iter() {
            if let HashMapEntry::Vacant(e) = shared_keys.entry(policy_id) {
                e.insert(keys);
            }
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

    pub async fn policy_by_id(&self, policy_id: EventId) -> Option<Policy> {
        let policies = self.policies.lock().await;
        policies.get(&policy_id).map(|pw| pw.policy.clone())
    }

    pub async fn policy_with_details(
        &self,
        policy_id: EventId,
    ) -> Option<(
        Policy,
        Option<Balance>,
        Option<Transactions>,
        Option<Timestamp>,
    )> {
        let policies = self.policies.lock().await;
        let descriptions = self.get_txs_descriptions().await;
        let pw = policies.get(&policy_id)?;
        let balance = pw.wallet.get_balance().ok();
        let list = pw.wallet.list_transactions(false).ok().map(|list| {
            list.into_iter()
                .map(|tx| {
                    let txid = tx.txid;
                    (tx, descriptions.get(&txid).cloned())
                })
                .collect()
        });
        Some((pw.policy.clone(), balance, list, pw.last_sync))
    }

    pub async fn cache_policy(
        &self,
        policy_id: EventId,
        policy: Policy,
        network: Network,
    ) -> Result<(), Error> {
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

    pub async fn proposals(&self) -> BTreeMap<EventId, (EventId, Proposal)> {
        let proposals = self.proposals.lock().await;
        proposals.clone()
    }

    pub async fn cache_proposal(
        &self,
        proposal_id: EventId,
        policy_id: EventId,
        proposal: Proposal,
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
                match map.get_mut(&author) {
                    Some(value) => {
                        if timestamp > value.2 {
                            value.0 = approved_proposal_id;
                            value.1 = psbt.clone();
                            value.2 = timestamp;
                            log::info!(
                                "Cached approved proposal {proposal_id} for pubkey {author} (updated)"
                            );
                        }
                    }
                    None => {
                        map.insert(author, (approved_proposal_id, psbt.clone(), timestamp));
                        log::info!(
                            "Cached approved proposal {proposal_id} for pubkey {author} (append)"
                        );
                    }
                };
            })
            .or_insert_with(|| {
                log::info!("Cached approved proposal {proposal_id} for pubkey {author}");
                [(author, (approved_proposal_id, psbt.clone(), timestamp))].into()
            });
    }

    pub async fn completed_proposal_exists(&self, completed_proposal_id: EventId) -> bool {
        let completed_proposals = self.completed_proposals.lock().await;
        completed_proposals.contains_key(&completed_proposal_id)
    }

    pub async fn completed_proposals(&self) -> BTreeMap<EventId, (EventId, CompletedProposal)> {
        let completed_proposals = self.completed_proposals.lock().await;
        completed_proposals.clone()
    }

    pub async fn cache_completed_proposal(
        &self,
        completed_proposal_id: EventId,
        policy_id: EventId,
        completed_proposal: CompletedProposal,
    ) {
        let mut completed_proposals = self.completed_proposals.lock().await;
        if let Entry::Vacant(e) = completed_proposals.entry(completed_proposal_id) {
            e.insert((policy_id, completed_proposal));
            log::info!("Cached completed proposal {completed_proposal_id}");
        }
    }

    pub async fn get_description_by_txid(&self, txid: Txid) -> Option<String> {
        let completed_proposals = self.completed_proposals.lock().await;
        for (_, (_, proposal)) in completed_proposals.iter() {
            if let CompletedProposal::Spending {
                txid: c_txid,
                description,
                ..
            } = proposal
            {
                if *c_txid == txid {
                    return Some(description.clone());
                }
            }
        }
        None
    }

    pub async fn get_txs_descriptions(&self) -> HashMap<Txid, String> {
        let completed_proposals = self.completed_proposals.lock().await;
        let mut map = HashMap::new();
        for (_, (_, proposal)) in completed_proposals.iter() {
            if let CompletedProposal::Spending {
                txid, description, ..
            } = proposal
            {
                if let HashMapEntry::Vacant(e) = map.entry(*txid) {
                    e.insert(description.clone());
                }
            }
        }
        map
    }

    pub async fn schedule_for_sync(&self, policy_id: EventId) {
        let mut policies = self.policies.lock().await;
        match policies.get_mut(&policy_id) {
            Some(pw) => {
                pw.last_sync = None;
            }
            None => log::error!("Policy {policy_id} not found"),
        }
    }

    pub async fn sync_with_timechain<B>(
        &self,
        blockchain: &B,
        sender: Option<&UnboundedSender<()>>,
        force: bool,
    ) -> Result<(), Error>
    where
        B: Blockchain,
    {
        if !self.block_height.is_synced().await {
            let block_height: u32 = blockchain.get_height()?;
            self.cache_block_height(block_height);
            self.block_height.just_synced().await;
        }

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

    pub async fn get_transactions(&self, policy_id: EventId) -> Option<Transactions> {
        let policies = self.policies.lock().await;
        let descriptions = self.get_txs_descriptions().await;
        let pw = policies.get(&policy_id)?;
        pw.wallet.list_transactions(false).ok().map(|list| {
            list.into_iter()
                .map(|tx| {
                    let txid = tx.txid;
                    (tx, descriptions.get(&txid).cloned())
                })
                .collect()
        })
    }

    pub async fn get_last_unused_address(&self, policy_id: EventId) -> Option<Address> {
        let policies = self.policies.lock().await;
        let pw = policies.get(&policy_id)?;
        pw.wallet
            .get_address(AddressIndex::LastUnused)
            .ok()
            .map(|a| a.address)
    }

    pub async fn get_total_balance(&self) -> Result<(Balance, bool), Error> {
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

    pub async fn get_all_transactions(
        &self,
    ) -> Result<Vec<(TransactionDetails, Option<String>)>, Error> {
        let policies = self.policies.lock().await;
        let descriptions = self.get_txs_descriptions().await;
        let mut transactions = Vec::new();
        let mut already_seen = Vec::new();
        for (_, pw) in policies.iter() {
            if !already_seen.contains(&&pw.policy.descriptor) {
                for tx in pw.wallet.list_transactions(false)?.into_iter() {
                    let desc: Option<String> = descriptions.get(&tx.txid).cloned();
                    transactions.push((tx, desc))
                }
                already_seen.push(&pw.policy.descriptor);
            }
        }
        Ok(transactions)
    }

    pub async fn get_tx(&self, txid: Txid) -> Option<(TransactionDetails, Option<String>)> {
        let policies = self.policies.lock().await;
        let desc = self.get_description_by_txid(txid).await;
        let mut already_seen = Vec::new();
        for (_, pw) in policies.iter() {
            if !already_seen.contains(&&pw.policy.descriptor) {
                let txs = pw.wallet.list_transactions(true).ok()?;
                for tx in txs.into_iter() {
                    if tx.txid == txid {
                        return Some((tx, desc));
                    }
                }
                already_seen.push(&pw.policy.descriptor);
            }
        }
        None
    }
}

impl CoinstrClient {
    pub fn sync<S>(
        &self,
        electrum_endpoint: S,
        sender: Option<UnboundedSender<()>>,
    ) -> JoinHandle<()>
    where
        S: Into<String>,
    {
        let this = self.clone();
        let electrum_endpoint: String = electrum_endpoint.into();
        tokio::task::spawn(async move {
            // Sync wallet thread
            let ccache = this.cache.clone();
            let ssender = sender.clone();
            let (abort_handle, abort_registration) = AbortHandle::new_pair();
            let wallet_sync = async move {
                let electrum_client = ElectrumClient::new(&electrum_endpoint).unwrap();
                let blockchain = ElectrumBlockchain::from(electrum_client);
                loop {
                    if let Err(e) = ccache
                        .sync_with_timechain(&blockchain, ssender.as_ref(), false)
                        .await
                    {
                        log::error!("Impossible to sync wallets: {e}");
                    }
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }
            };
            let future = Abortable::new(wallet_sync, abort_registration);
            tokio::task::spawn(async {
                let _ = future.await;
                log::debug!("Exited from wallet sync thread");
            });

            let keys = this.keys();

            let shared_keys = this
                .get_shared_keys(Some(Duration::from_secs(60)))
                .await
                .unwrap_or_default();
            this.cache.cache_shared_keys(shared_keys).await;

            log::info!("Got shared keys");

            let filters = vec![
                Filter::new().pubkey(keys.public_key()).kind(POLICY_KIND),
                Filter::new().pubkey(keys.public_key()).kind(PROPOSAL_KIND),
                Filter::new()
                    .pubkey(keys.public_key())
                    .kind(APPROVED_PROPOSAL_KIND)
                    .since(Timestamp::now().sub(APPROVED_PROPOSAL_EXPIRATION)),
                Filter::new()
                    .pubkey(keys.public_key())
                    .kind(COMPLETED_PROPOSAL_KIND),
            ];

            this.client.subscribe(filters).await;
            let _ = this
                .client
                .handle_notifications(|notification| async {
                    match notification {
                        RelayPoolNotification::Event(_, event) => {
                            let event_id = event.id;
                            if event.is_expired() {
                                log::warn!("Event {event_id} expired");
                            } else {
                                match this.handle_event(event).await {
                                    Ok(_) => {
                                        if let Some(sender) = sender.as_ref() {
                                            sender.send(()).ok();
                                        }
                                    }
                                    Err(e) => {
                                        log::error!("Impossible to handle event {event_id}: {e}")
                                    }
                                }
                            }
                        }
                        RelayPoolNotification::Shutdown => {
                            abort_handle.abort();
                        }
                        _ => (),
                    }

                    Ok(())
                })
                .await;
            log::debug!("Exited from nostr sync thread");
        })
    }

    #[async_recursion]
    async fn handle_event(&self, event: Event) -> Result<()> {
        if event.kind == POLICY_KIND && !self.cache.policy_exists(event.id).await {
            if let Some(shared_key) = self.cache.shared_key_by_policy_id(event.id).await {
                let policy = Policy::decrypt(&shared_key, &event.content)?;
                self.cache
                    .cache_policy(event.id, policy, self.network())
                    .await?;
            } else {
                log::info!("Requesting shared key for {}", event.id);
                tokio::time::sleep(Duration::from_secs(1)).await;
                let shared_key = self
                    .get_shared_key_by_policy_id(event.id, Some(Duration::from_secs(30)))
                    .await?;
                self.cache.cache_shared_key(event.id, shared_key).await;
                self.handle_event(event).await?;
            }
        } else if event.kind == PROPOSAL_KIND && !self.cache.proposal_exists(event.id).await {
            if let Some(policy_id) = util::extract_first_event_id(&event) {
                if let Some(shared_key) = self.cache.shared_key_by_policy_id(policy_id).await {
                    let proposal = Proposal::decrypt(&shared_key, &event.content)?;
                    self.cache
                        .cache_proposal(event.id, policy_id, proposal)
                        .await;
                } else {
                    log::info!("Requesting shared key for proposal {}", event.id);
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    let shared_key = self
                        .get_shared_key_by_policy_id(policy_id, Some(Duration::from_secs(30)))
                        .await?;
                    self.cache.cache_shared_key(policy_id, shared_key).await;
                    self.handle_event(event).await?;
                }
            } else {
                log::error!("Impossible to find policy id in proposal {}", event.id);
            }
        } else if event.kind == APPROVED_PROPOSAL_KIND {
            if let Some(proposal_id) = util::extract_first_event_id(&event) {
                if let Some(Tag::Event(policy_id, ..)) =
                    util::extract_tags_by_kind(&event, TagKind::E).get(1)
                {
                    if let Some(shared_key) = self.cache.shared_key_by_policy_id(*policy_id).await {
                        let approved_proposal =
                            ApprovedProposal::decrypt(&shared_key, &event.content)?;
                        self.cache
                            .cache_approved_proposal(
                                proposal_id,
                                event.pubkey,
                                event.id,
                                approved_proposal.psbt(),
                                event.created_at,
                            )
                            .await;
                    } else {
                        log::info!("Requesting shared key for approved proposal {}", event.id);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        let shared_key = self
                            .get_shared_key_by_policy_id(*policy_id, Some(Duration::from_secs(30)))
                            .await?;
                        self.cache.cache_shared_key(*policy_id, shared_key).await;
                        self.handle_event(event).await?;
                    }
                } else {
                    log::error!("Impossible to find policy id in proposal {}", event.id);
                }
            } else {
                log::error!(
                    "Impossible to find proposal id in approved proposal {}",
                    event.id
                );
            }
        } else if event.kind == COMPLETED_PROPOSAL_KIND {
            if let Some(proposal_id) = util::extract_first_event_id(&event) {
                self.cache.uncache_proposal(proposal_id).await;
                if let Some(Tag::Event(policy_id, ..)) =
                    util::extract_tags_by_kind(&event, TagKind::E).get(1)
                {
                    // Schedule policy for sync if the event was created in the last 60 secs
                    if event.created_at.add(Duration::from_secs(60)) >= Timestamp::now() {
                        self.cache.schedule_for_sync(*policy_id).await;
                    }

                    if let Some(shared_key) = self.cache.shared_key_by_policy_id(*policy_id).await {
                        let completed_proposal =
                            CompletedProposal::decrypt(&shared_key, &event.content)?;
                        self.cache
                            .cache_completed_proposal(event.id, *policy_id, completed_proposal)
                            .await;
                    } else {
                        log::info!("Requesting shared key for completed proposal {}", event.id);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                        let shared_key = self
                            .get_shared_key_by_policy_id(*policy_id, Some(Duration::from_secs(30)))
                            .await?;
                        self.cache.cache_shared_key(*policy_id, shared_key).await;
                        self.handle_event(event).await?;
                    }
                } else {
                    log::error!(
                        "Impossible to find policy id in completed proposal {}",
                        event.id
                    );
                }
            }
        }

        Ok(())
    }
}