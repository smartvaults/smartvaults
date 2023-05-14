// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//! Store

#![allow(clippy::type_complexity)]

use std::collections::btree_map::Entry;
use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::path::Path;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, Txid};
use bdk::blockchain::Blockchain;
use bdk::wallet::AddressIndex;
use bdk::{Balance, SyncOptions, TransactionDetails, Wallet};
use keechain_core::util::serde::{deserialize, serialize};
use nostr_sdk::event::id::{self, EventId};
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::Keys;
use nostr_sdk::Timestamp;
use parking_lot::Mutex;
use sled::{Db, Tree};
use tokio::sync::mpsc::Sender;

use crate::policy::{self, Policy};
use crate::proposal::{CompletedProposal, Proposal};

const BLOCK_HEIGHT_SYNC_INTERVAL: Duration = Duration::from_secs(60);
const WALLET_SYNC_INTERVAL: Duration = Duration::from_secs(60);

pub type Transactions = Vec<(TransactionDetails, Option<String>)>;
type ApprovedProposals =
    BTreeMap<EventId, BTreeMap<XOnlyPublicKey, (EventId, PartiallySignedTransaction, Timestamp)>>;

/// Store error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Sled error
    #[error(transparent)]
    Sled(#[from] sled::Error),
    /// Bdk error
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    /// Json error
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Keys error
    #[error(transparent)]
    Keys(#[from] nostr_sdk::nostr::key::Error),
    /// EventId error
    #[error(transparent)]
    EventId(#[from] id::Error),
    /// Secp256k1 error
    #[error(transparent)]
    Secp256k1(#[from] nostr_sdk::secp256k1::Error),
    /// Policy error
    #[error(transparent)]
    Policy(#[from] policy::Error),
    /// Not found
    #[error("not found")]
    NotFound,
    /// Wallet ot found
    #[error("wallet not found")]
    WalletNotFound,
}

pub struct GetApprovedProposals {
    pub policy_id: EventId,
    pub proposal: Proposal,
    pub signed_psbts: Vec<PartiallySignedTransaction>,
    pub approvals: Vec<XOnlyPublicKey>,
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

    pub fn is_synced(&self) -> bool {
        let last_sync = self.last_sync.lock();
        let last_sync: Timestamp = last_sync.unwrap_or_else(|| Timestamp::from(0));
        last_sync.add(BLOCK_HEIGHT_SYNC_INTERVAL) > Timestamp::now()
    }

    pub fn just_synced(&self) {
        let mut last_sync = self.last_sync.lock();
        *last_sync = Some(Timestamp::now());
    }
}

/// Store
#[derive(Debug, Clone)]
pub struct Store {
    //nostr_db: Db,
    shared_keys: Tree,
    nostr_public_keys: Tree,
    policies: Tree,
    proposals: Tree,
    approved_proposals: Arc<Mutex<ApprovedProposals>>,
    completed_proposals: Tree,
    timechain_db: Db,
    block_height: BlockHeight,
    wallets: Arc<Mutex<BTreeMap<EventId, (Wallet<Tree>, Option<Timestamp>)>>>,
    network: Network,
    // cache: Cache,
}

impl Drop for Store {
    fn drop(&mut self) {}
}

impl Store {
    /// Open new database
    pub fn open<P>(nostr_db_path: P, timechain_db_path: P, network: Network) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let nostr_db = sled::open(nostr_db_path)?;
        let timechain_db = sled::open(timechain_db_path)?;
        let shared_keys = nostr_db.open_tree("shared_keys")?;
        let nostr_public_keys = nostr_db.open_tree("nostr_public_keys")?;
        let policies = nostr_db.open_tree("policies")?;
        let proposals = nostr_db.open_tree("proposals")?;
        let approved_proposals = Arc::new(Mutex::new(BTreeMap::new()));
        let completed_proposals = nostr_db.open_tree("completed_proposals")?;

        Ok(Self {
            //nostr_db,
            timechain_db,
            shared_keys,
            nostr_public_keys,
            policies,
            proposals,
            approved_proposals,
            completed_proposals,
            block_height: BlockHeight::default(),
            wallets: Arc::new(Mutex::new(BTreeMap::new())),
            network,
        })
    }

    pub fn load_wallets(&self) -> Result<(), Error> {
        let mut wallets = self.wallets.lock();
        for (policy_id, policy) in self.get_policies()? {
            let tree: Tree = self.timechain_db.open_tree(policy_id.to_hex())?;
            wallets.insert(
                policy_id,
                (
                    Wallet::new(&policy.descriptor.to_string(), None, self.network, tree)?,
                    None,
                ),
            );
        }
        Ok(())
    }

    /// Close db
    pub fn close(self) {
        drop(self);
    }

    pub fn block_height(&self) -> u32 {
        self.block_height.block_height()
    }

    pub fn save_shared_key(&self, policy_id: EventId, shared_key: Keys) -> Result<(), Error> {
        self.shared_keys.insert(
            policy_id,
            serialize(shared_key.secret_key()?.display_secret().to_string())?,
        )?;
        Ok(())
    }

    pub fn get_shared_key(&self, policy_id: EventId) -> Result<Keys, Error> {
        let sk: Vec<u8> = self
            .shared_keys
            .get(policy_id)?
            .ok_or(Error::NotFound)?
            .to_vec();
        Ok(Keys::new(deserialize(sk)?))
    }

    pub fn delete_shared_key(&self, policy_id: EventId) -> Result<(), Error> {
        self.shared_keys.remove(policy_id)?;
        log::info!("Deleted shared key for policy {policy_id}");
        Ok(())
    }

    pub fn policy_exists(&self, policy_id: EventId) -> Result<bool, Error> {
        Ok(self.policies.contains_key(policy_id)?)
    }

    pub fn save_policy(
        &self,
        policy_id: EventId,
        policy: Policy,
        nostr_public_keys: Vec<XOnlyPublicKey>,
    ) -> Result<(), Error> {
        let descriptor = policy.descriptor.to_string();
        self.policies.insert(policy_id, serialize(policy)?)?;
        self.nostr_public_keys
            .insert(policy_id, serialize(nostr_public_keys)?)?;

        // Load wallet
        let mut wallets = self.wallets.lock();
        if let Entry::Vacant(e) = wallets.entry(policy_id) {
            let tree: Tree = self.timechain_db.open_tree(policy_id.to_hex())?;
            e.insert((Wallet::new(&descriptor, None, self.network, tree)?, None));
        }

        log::info!("Policy {policy_id} saved");
        Ok(())
    }

    pub fn get_policy(&self, policy_id: EventId) -> Result<Policy, Error> {
        let policy: Vec<u8> = self
            .policies
            .get(policy_id)?
            .ok_or(Error::NotFound)?
            .to_vec();
        Ok(deserialize(policy)?)
    }

    pub fn get_policies(&self) -> Result<BTreeMap<EventId, Policy>, Error> {
        let mut policies = BTreeMap::new();
        for res in self.policies.into_iter() {
            let (policy_id, policy) = res?;
            policies.insert(
                EventId::from_slice(&policy_id)?,
                deserialize(policy.to_vec())?,
            );
        }
        Ok(policies)
    }

    pub fn delete_policy(&self, policy_id: EventId) -> Result<(), Error> {
        self.policies.remove(policy_id)?;
        log::info!("Deleted policy {policy_id}");
        Ok(())
    }

    pub fn policies_with_balance(
        &self,
    ) -> Result<BTreeMap<EventId, (Policy, Option<Balance>, bool)>, Error> {
        let wallets = self.wallets.lock();
        let mut new_policies = BTreeMap::new();
        for (policy_id, policy) in self.get_policies()?.into_iter() {
            let (wallet, last_sync) = wallets.get(&policy_id).ok_or(Error::WalletNotFound)?;
            new_policies.insert(
                policy_id,
                (policy, wallet.get_balance().ok(), last_sync.is_some()),
            );
        }
        Ok(new_policies)
    }

    pub fn get_nostr_pubkeys(&self, policy_id: EventId) -> Result<Vec<XOnlyPublicKey>, Error> {
        let pubkeys: Vec<u8> = self
            .nostr_public_keys
            .get(policy_id)?
            .ok_or(Error::NotFound)?
            .to_vec();
        Ok(deserialize(pubkeys)?)
    }

    pub fn policy_with_details(
        &self,
        policy_id: EventId,
    ) -> Option<(
        Policy,
        Option<Balance>,
        Option<Transactions>,
        Option<Timestamp>,
    )> {
        let policy: Policy = self.get_policy(policy_id).ok()?;
        let wallets = self.wallets.lock();
        let descriptions = self.get_txs_descriptions().ok()?;
        let (wallet, last_sync) = wallets.get(&policy_id)?;
        let balance = wallet.get_balance().ok();
        let list = wallet.list_transactions(false).ok().map(|list| {
            list.into_iter()
                .map(|tx| {
                    let txid = tx.txid;
                    (tx, descriptions.get(&txid).cloned())
                })
                .collect()
        });
        Some((policy, balance, list, *last_sync))
    }

    pub fn proposal_exists(&self, proposal_id: EventId) -> Result<bool, Error> {
        Ok(self.proposals.contains_key(proposal_id)?)
    }

    pub fn get_proposals(&self) -> Result<BTreeMap<EventId, (EventId, Proposal)>, Error> {
        let mut proposals = BTreeMap::new();
        for res in self.proposals.into_iter() {
            let (proposal_id, tuple) = res?;
            proposals.insert(
                EventId::from_slice(&proposal_id)?,
                deserialize(tuple.to_vec())?,
            );
        }
        Ok(proposals)
    }

    pub fn get_proposal(&self, proposal_id: EventId) -> Result<(EventId, Proposal), Error> {
        let tuple: Vec<u8> = self
            .proposals
            .get(proposal_id)?
            .ok_or(Error::NotFound)?
            .to_vec();
        Ok(deserialize(tuple)?)
    }

    pub fn save_proposal(
        &self,
        proposal_id: EventId,
        policy_id: EventId,
        proposal: Proposal,
    ) -> Result<(), Error> {
        self.proposals
            .insert(proposal_id, serialize((policy_id, proposal))?)?;
        log::info!("Spending proposal {proposal_id} saved");
        Ok(())
    }

    pub fn delete_proposal(&self, proposal_id: EventId) -> Result<(), Error> {
        self.proposals.remove(proposal_id)?;
        let mut approved_proposals = self.approved_proposals.lock();
        approved_proposals.remove(&proposal_id);
        log::info!("Deleted proposal {proposal_id}");
        Ok(())
    }

    pub fn approved_proposals(&self) -> ApprovedProposals {
        let approved_proposals = self.approved_proposals.lock();
        approved_proposals.clone()
    }

    pub fn signed_psbts_by_proposal_id(
        &self,
        proposal_id: EventId,
    ) -> Option<BTreeMap<XOnlyPublicKey, (EventId, PartiallySignedTransaction, Timestamp)>> {
        let approved_proposals = self.approved_proposals.lock();
        approved_proposals.get(&proposal_id).cloned()
    }

    pub fn save_approved_proposal(
        &self,
        proposal_id: EventId,
        author: XOnlyPublicKey,
        approved_proposal_id: EventId,
        psbt: PartiallySignedTransaction,
        timestamp: Timestamp,
    ) {
        let mut approved_proposals = self.approved_proposals.lock();
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

    pub fn get_approved_proposals_by_id(
        &self,
        proposal_id: EventId,
    ) -> Result<GetApprovedProposals, Error> {
        let (policy_id, proposal) = self.get_proposal(proposal_id)?;
        let approved_proposals = self.approved_proposals.lock();
        let data = approved_proposals
            .get(&proposal_id)
            .ok_or(Error::NotFound)?;

        let mut signed_psbts = Vec::new();
        let mut approvals = Vec::new();

        for (pubkey, (_, psbt, ..)) in data.iter() {
            signed_psbts.push(psbt.clone());
            approvals.push(*pubkey);
        }

        Ok(GetApprovedProposals {
            policy_id,
            proposal,
            signed_psbts,
            approvals,
        })
    }

    pub fn completed_proposal_exists(&self, completed_proposal_id: EventId) -> Result<bool, Error> {
        Ok(self
            .completed_proposals
            .contains_key(completed_proposal_id)?)
    }

    pub fn completed_proposals(
        &self,
    ) -> Result<BTreeMap<EventId, (EventId, CompletedProposal)>, Error> {
        let mut completed_proposals = BTreeMap::new();
        for res in self.completed_proposals.into_iter() {
            let (proposal_id, tuple) = res?;
            completed_proposals.insert(
                EventId::from_slice(&proposal_id)?,
                deserialize(tuple.to_vec())?,
            );
        }
        Ok(completed_proposals)
    }

    pub fn save_completed_proposal(
        &self,
        completed_proposal_id: EventId,
        policy_id: EventId,
        completed_proposal: CompletedProposal,
    ) -> Result<(), Error> {
        self.proposals.insert(
            completed_proposal_id,
            serialize((policy_id, completed_proposal))?,
        )?;
        log::info!("Completed proposal {completed_proposal_id} saved");
        Ok(())
    }

    pub fn get_completed_proposal(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<(EventId, CompletedProposal), Error> {
        let tuple: Vec<u8> = self
            .completed_proposals
            .get(completed_proposal_id)?
            .ok_or(Error::NotFound)?
            .to_vec();
        Ok(deserialize(tuple)?)
    }

    pub fn delete_completed_proposal(&self, completed_proposal_id: EventId) -> Result<(), Error> {
        self.completed_proposals.remove(completed_proposal_id)?;
        log::info!("Deleted completed proposal {completed_proposal_id}");
        Ok(())
    }

    fn get_description_by_txid(&self, txid: Txid) -> Result<Option<String>, Error> {
        for res in self.completed_proposals.into_iter() {
            let (_, tuple) = res?;
            let (_, proposal): (EventId, CompletedProposal) = deserialize(tuple.to_vec())?;
            if let CompletedProposal::Spending {
                txid: c_txid,
                description,
                ..
            } = proposal
            {
                if c_txid == txid {
                    return Ok(Some(description));
                }
            }
        }
        Ok(None)
    }

    pub fn get_txs_descriptions(&self) -> Result<HashMap<Txid, String>, Error> {
        let mut map = HashMap::new();
        for res in self.completed_proposals.into_iter() {
            let (_, tuple) = res?;
            let (_, proposal): (EventId, CompletedProposal) = deserialize(tuple.to_vec())?;
            if let CompletedProposal::Spending {
                txid, description, ..
            } = proposal
            {
                if let HashMapEntry::Vacant(e) = map.entry(txid) {
                    e.insert(description);
                }
            }
        }
        Ok(map)
    }

    pub fn get_balance(&self, policy_id: EventId) -> Option<Balance> {
        let wallets = self.wallets.lock();
        let (wallet, ..) = wallets.get(&policy_id)?;
        wallet.get_balance().ok()
    }

    pub fn get_transactions(&self, policy_id: EventId) -> Option<Transactions> {
        let wallets = self.wallets.lock();
        let descriptions = self.get_txs_descriptions().ok()?;
        let (wallet, ..) = wallets.get(&policy_id)?;
        wallet.list_transactions(false).ok().map(|list| {
            list.into_iter()
                .map(|tx| {
                    let txid = tx.txid;
                    (tx, descriptions.get(&txid).cloned())
                })
                .collect()
        })
    }

    pub fn get_last_unused_address(&self, policy_id: EventId) -> Option<Address> {
        let wallets = self.wallets.lock();
        let (wallet, ..) = wallets.get(&policy_id)?;
        wallet
            .get_address(AddressIndex::LastUnused)
            .ok()
            .map(|a| a.address)
    }

    pub fn get_total_balance(&self) -> Result<(Balance, bool), Error> {
        let wallets = self.wallets.lock();
        let mut synced = true;
        let mut total_balance = Balance::default();
        let mut already_seen = Vec::new();
        for (policy_id, (wallet, last_sync)) in wallets.iter() {
            let policy: Policy = self.get_policy(*policy_id)?;
            if !already_seen.contains(&policy.descriptor) {
                if last_sync.is_none() {
                    synced = false;
                    break;
                }
                let balance = wallet.get_balance()?;
                total_balance = total_balance.add(balance);
                already_seen.push(policy.descriptor);
            }
        }
        Ok((total_balance, synced))
    }

    pub fn get_all_transactions(&self) -> Result<Vec<(TransactionDetails, Option<String>)>, Error> {
        let wallets = self.wallets.lock();
        let descriptions = self.get_txs_descriptions()?;
        let mut transactions = Vec::new();
        let mut already_seen = Vec::new();
        for (policy_id, (wallet, ..)) in wallets.iter() {
            let policy: Policy = self.get_policy(*policy_id)?;
            if !already_seen.contains(&policy.descriptor) {
                for tx in wallet.list_transactions(false)?.into_iter() {
                    let desc: Option<String> = descriptions.get(&tx.txid).cloned();
                    transactions.push((tx, desc))
                }
                already_seen.push(policy.descriptor);
            }
        }
        Ok(transactions)
    }

    pub fn get_tx(&self, txid: Txid) -> Option<(TransactionDetails, Option<String>)> {
        let wallets = self.wallets.lock();
        let desc = self.get_description_by_txid(txid).ok()?;
        let mut already_seen = Vec::new();
        for (policy_id, (wallet, ..)) in wallets.iter() {
            let policy: Policy = self.get_policy(*policy_id).ok()?;
            if !already_seen.contains(&policy.descriptor) {
                let txs = wallet.list_transactions(true).ok()?;
                for tx in txs.into_iter() {
                    if tx.txid == txid {
                        return Some((tx, desc));
                    }
                }
                already_seen.push(policy.descriptor);
            }
        }
        None
    }

    pub fn schedule_for_sync(&self, policy_id: EventId) {
        let mut wallets = self.wallets.lock();
        match wallets.get_mut(&policy_id) {
            Some((_, last_sync)) => {
                *last_sync = None;
            }
            None => log::error!("Wallet for policy {policy_id} not found"),
        }
    }

    pub fn sync_with_timechain<B>(
        &self,
        blockchain: &B,
        sender: Option<&Sender<()>>,
        force: bool,
    ) -> Result<(), Error>
    where
        B: Blockchain,
    {
        if !self.block_height.is_synced() {
            let block_height: u32 = blockchain.get_height()?;
            self.block_height.set_block_height(block_height);
            self.block_height.just_synced();
        }

        let mut wallets = self.wallets.lock();
        for (policy_id, (wallet, last_sync)) in wallets.iter_mut() {
            if force
                || last_sync
                    .unwrap_or_else(|| Timestamp::from(0))
                    .add(WALLET_SYNC_INTERVAL)
                    <= Timestamp::now()
            {
                log::info!("Syncing policy {policy_id}");
                wallet.sync(blockchain, SyncOptions::default())?;
                *last_sync = Some(Timestamp::now());
                if let Some(sender) = sender {
                    let _ = sender.try_send(());
                }
            }
        }
        Ok(())
    }

    pub fn delete_generic_event_id(&self, event_id: EventId) -> Result<(), Error> {
        if self.policy_exists(event_id)? {
            self.delete_policy(event_id)?;
        } else if self.proposal_exists(event_id)? {
            self.delete_proposal(event_id)?;
        } else if self.completed_proposal_exists(event_id)? {
            self.delete_completed_proposal(event_id)?;
        };

        Ok(())
    }
}
