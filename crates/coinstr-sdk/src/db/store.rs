// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//! Store

#![allow(clippy::type_complexity)]

use std::collections::btree_map::Entry;
use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;

use bdk::bitcoin::{Address, Network, Txid};
use bdk::blockchain::Blockchain;
use bdk::database::SqliteDatabase;
use bdk::wallet::AddressIndex;
use bdk::{Balance, SyncOptions, TransactionDetails, Wallet};
use coinstr_core::policy::{self, Policy};
use coinstr_core::proposal::{CompletedProposal, Proposal};
use coinstr_core::ApprovedProposal;
use nostr_sdk::event::id::{self, EventId};
use nostr_sdk::secp256k1::{SecretKey, XOnlyPublicKey};
use nostr_sdk::{Keys, Timestamp, Url};
use parking_lot::Mutex;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::OpenFlags;
use tokio::sync::mpsc::Sender;

use super::migration::{self, MigrationError, STARTUP_SQL};
use super::model::{GetDetailedPolicyResult, GetPolicyResult};
use crate::util::encryption::{EncryptionWithKeys, EncryptionWithKeysError};

const BLOCK_HEIGHT_SYNC_INTERVAL: Duration = Duration::from_secs(60);
const WALLET_SYNC_INTERVAL: Duration = Duration::from_secs(60);

pub(crate) type SqlitePool = r2d2::Pool<SqliteConnectionManager>;
pub(crate) type PooledConnection = r2d2::PooledConnection<SqliteConnectionManager>;
pub type Transactions = Vec<(TransactionDetails, Option<String>)>;
type ApprovedProposals =
    BTreeMap<EventId, BTreeMap<XOnlyPublicKey, (EventId, ApprovedProposal, Timestamp)>>;

/// Store error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Sqlite error
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    /// Sqlite Pool error
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
    /// Migration error
    #[error(transparent)]
    Migration(#[from] MigrationError),
    /// Bdk error
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    /// Encryption error
    #[error(transparent)]
    EncryptionWithKeys(#[from] EncryptionWithKeysError),
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
    #[error("impossible to open policy {0} db")]
    FailedToOpenPolicyDb(EventId),
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
    pub approved_proposals: Vec<ApprovedProposal>,
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
    pool: SqlitePool,
    keys: Keys,
    approved_proposals: Arc<Mutex<ApprovedProposals>>,
    block_height: BlockHeight,
    wallets: Arc<Mutex<BTreeMap<EventId, Wallet<SqliteDatabase>>>>,
    timechain_db_path: PathBuf,
    network: Network,
}

impl Drop for Store {
    fn drop(&mut self) {}
}

impl Store {
    /// Open new database
    pub fn open<P>(
        user_db_path: P,
        timechain_db_path: P,
        keys: &Keys,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let manager = SqliteConnectionManager::file(user_db_path.as_ref())
            .with_flags(OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)
            .with_init(|c| c.execute_batch(STARTUP_SQL));
        let pool = r2d2::Pool::new(manager)?;
        migration::run(&mut pool.get()?)?;
        let approved_proposals = Arc::new(Mutex::new(BTreeMap::new()));
        Ok(Self {
            pool,
            keys: keys.clone(),
            wallets: Arc::new(Mutex::new(BTreeMap::new())),
            approved_proposals,
            block_height: BlockHeight::default(),
            timechain_db_path: timechain_db_path.as_ref().to_path_buf(),
            network,
        })
    }

    pub(crate) fn get_wallet_db(&self, policy_id: EventId) -> Result<SqliteDatabase, Error> {
        let path = self.timechain_db_path.clone();
        let handle =
            std::thread::spawn(move || SqliteDatabase::new(path.join(format!("{policy_id}.db"))));
        handle
            .join()
            .map_err(|_| Error::FailedToOpenPolicyDb(policy_id))
    }

    pub fn load_wallets(&self) -> Result<(), Error> {
        let mut wallets = self.wallets.lock();
        for (policy_id, GetPolicyResult { policy, .. }) in self.get_policies()? {
            let db: SqliteDatabase = self.get_wallet_db(policy_id)?;
            wallets.insert(
                policy_id,
                Wallet::new(&policy.descriptor.to_string(), None, self.network, db)?,
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

    pub fn policy_exists(&self, policy_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT EXISTS(SELECT 1 FROM policies WHERE policy_id = ? LIMIT 1);")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_policy(
        &self,
        policy_id: EventId,
        policy: Policy,
        nostr_public_keys: Vec<XOnlyPublicKey>,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO policies (policy_id, policy) VALUES (?, ?);",
            (policy_id.to_hex(), policy.encrypt_with_keys(&self.keys)?),
        )?;
        // Save nostr public keys
        let mut stmt = conn.prepare(
            "INSERT OR IGNORE INTO nostr_public_keys (policy_id, public_key) VALUES (?, ?);",
        )?;
        for public_key in nostr_public_keys.into_iter() {
            stmt.execute((policy_id.to_hex(), public_key.to_string()))?;
        }
        // Load wallet
        let mut wallets = self.wallets.lock();
        if let Entry::Vacant(e) = wallets.entry(policy_id) {
            let db = SqliteDatabase::new(self.timechain_db_path.join(format!("{policy_id}.db")));
            e.insert(Wallet::new(
                &policy.descriptor.to_string(),
                None,
                self.network,
                db,
            )?);
        }
        Ok(())
    }

    pub fn get_policy(&self, policy_id: EventId) -> Result<GetPolicyResult, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT policy, last_sync FROM policies WHERE policy_id = ?")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound)?;
        let policy: String = row.get(0)?;
        let last_sync: Option<u64> = row.get(1)?;
        Ok(GetPolicyResult {
            policy: Policy::decrypt_with_keys(&self.keys, policy)?,
            last_sync: last_sync.map(Timestamp::from),
        })
    }

    pub fn get_last_sync(&self, policy_id: EventId) -> Result<Option<Timestamp>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT last_sync FROM policies WHERE policy_id = ?")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound)?;
        let last_sync: Option<u64> = row.get(0)?;
        Ok(last_sync.map(Timestamp::from))
    }

    pub fn update_last_sync(
        &self,
        policy_id: EventId,
        last_sync: Option<Timestamp>,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("UPDATE policies SET last_sync = ? WHERE policy_id = ?")?;
        stmt.execute((last_sync.map(|t| t.as_u64()), policy_id.to_hex()))?;
        Ok(())
    }

    pub fn get_policies(&self) -> Result<BTreeMap<EventId, GetPolicyResult>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT policy_id, policy, last_sync FROM policies")?;
        let mut rows = stmt.query([])?;
        let mut policies = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let policy_id: String = row.get(0)?;
            let policy: String = row.get(1)?;
            let last_sync: Option<u64> = row.get(2)?;
            policies.insert(
                EventId::from_hex(policy_id)?,
                GetPolicyResult {
                    policy: Policy::decrypt_with_keys(&self.keys, policy)?,
                    last_sync: last_sync.map(Timestamp::from),
                },
            );
        }
        Ok(policies)
    }

    pub fn get_detailed_policies(
        &self,
    ) -> Result<BTreeMap<EventId, GetDetailedPolicyResult>, Error> {
        let mut policies = BTreeMap::new();
        for (policy_id, GetPolicyResult { policy, last_sync }) in self.get_policies()?.into_iter() {
            policies.insert(
                policy_id,
                GetDetailedPolicyResult {
                    policy,
                    balance: self.get_balance(policy_id),
                    last_sync,
                },
            );
        }
        Ok(policies)
    }

    pub fn delete_policy(&self, policy_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM policies WHERE policy_id = ?;",
            [policy_id.to_hex()],
        )?;
        conn.execute(
            "DELETE FROM nostr_public_keys WHERE policy_id = ?;",
            [policy_id.to_hex()],
        )?;
        conn.execute(
            "DELETE FROM shared_keys WHERE policy_id = ?;",
            [policy_id.to_hex()],
        )?;
        let mut wallets = self.wallets.lock();
        wallets.remove(&policy_id);
        log::info!("Deleted shared key for policy {policy_id}");
        Ok(())
    }

    pub fn save_shared_key(&self, policy_id: EventId, shared_key: Keys) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO shared_keys (policy_id, shared_key) VALUES (?, ?);",
            (
                policy_id.to_hex(),
                shared_key.secret_key()?.encrypt_with_keys(&self.keys)?,
            ),
        )?;
        Ok(())
    }

    pub fn get_shared_key(&self, policy_id: EventId) -> Result<Keys, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT shared_key FROM shared_keys WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound)?;
        let sk: String = row.get(0)?;
        let sk = SecretKey::decrypt_with_keys(&self.keys, sk)?;
        Ok(Keys::new(sk))
    }

    pub fn get_nostr_pubkeys(&self, policy_id: EventId) -> Result<Vec<XOnlyPublicKey>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT public_key FROM nostr_public_keys WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut pubkeys = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let public_key: String = row.get(0)?;
            pubkeys.push(XOnlyPublicKey::from_str(&public_key)?);
        }
        Ok(pubkeys)
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
        let GetPolicyResult { policy, last_sync } = self.get_policy(policy_id).ok()?;
        let wallets = self.wallets.lock();
        let descriptions = self.get_txs_descriptions().ok()?;
        let wallet = wallets.get(&policy_id)?;
        let balance = wallet.get_balance().ok();
        let list = wallet.list_transactions(false).ok().map(|list| {
            list.into_iter()
                .map(|tx| {
                    let txid = tx.txid;
                    (tx, descriptions.get(&txid).cloned())
                })
                .collect()
        });
        Some((policy, balance, list, last_sync))
    }

    pub fn proposal_exists(&self, proposal_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM proposals WHERE proposal_id = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([proposal_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_proposal(
        &self,
        proposal_id: EventId,
        policy_id: EventId,
        proposal: Proposal,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO proposals (proposal_id, policy_id, proposal) VALUES (?, ?, ?);",
            (
                proposal_id.to_hex(),
                policy_id.to_hex(),
                proposal.encrypt_with_keys(&self.keys)?,
            ),
        )?;
        log::info!("Spending proposal {proposal_id} saved");
        Ok(())
    }

    pub fn get_proposals(&self) -> Result<BTreeMap<EventId, (EventId, Proposal)>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT proposal_id, policy_id, proposal FROM proposals;")?;
        let mut rows = stmt.query([])?;
        let mut proposals = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let proposal_id: String = row.get(0)?;
            let policy_id: String = row.get(1)?;
            let proposal: String = row.get(2)?;
            proposals.insert(
                EventId::from_hex(proposal_id)?,
                (
                    EventId::from_hex(policy_id)?,
                    Proposal::decrypt_with_keys(&self.keys, proposal)?,
                ),
            );
        }
        Ok(proposals)
    }

    pub fn get_proposal(&self, proposal_id: EventId) -> Result<(EventId, Proposal), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT policy_id, proposal FROM proposals WHERE proposal_id = ? LIMIT 1;",
        )?;
        let mut rows = stmt.query([proposal_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound)?;
        let policy_id: String = row.get(0)?;
        let proposal: String = row.get(1)?;
        Ok((
            EventId::from_hex(policy_id)?,
            Proposal::decrypt_with_keys(&self.keys, proposal)?,
        ))
    }

    pub fn delete_proposal(&self, proposal_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM proposals WHERE proposal_id = ?;",
            [proposal_id.to_hex()],
        )?;
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
    ) -> Option<BTreeMap<XOnlyPublicKey, (EventId, ApprovedProposal, Timestamp)>> {
        let approved_proposals = self.approved_proposals.lock();
        approved_proposals.get(&proposal_id).cloned()
    }

    pub fn save_approved_proposal(
        &self,
        proposal_id: EventId,
        author: XOnlyPublicKey,
        approved_proposal_id: EventId,
        approved_proposal: ApprovedProposal,
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
                            value.1 = approved_proposal.clone();
                            value.2 = timestamp;
                            log::info!(
                                "Cached approved proposal {proposal_id} for pubkey {author} (updated)"
                            );
                        }
                    }
                    None => {
                        map.insert(author, (approved_proposal_id, approved_proposal.clone(), timestamp));
                        log::info!(
                            "Cached approved proposal {proposal_id} for pubkey {author} (append)"
                        );
                    }
                };
            })
            .or_insert_with(|| {
                log::info!("Cached approved proposal {proposal_id} for pubkey {author}");
                [(author, (approved_proposal_id, approved_proposal.clone(), timestamp))].into()
            });
    }

    pub fn get_approved_proposals_by_id(
        &self,
        proposal_id: EventId,
    ) -> Result<GetApprovedProposals, Error> {
        let (policy_id, proposal) = self.get_proposal(proposal_id)?;
        let approved_proposals = self.approved_proposals.lock();
        let proposals = approved_proposals
            .get(&proposal_id)
            .ok_or(Error::NotFound)?;
        Ok(GetApprovedProposals {
            policy_id,
            proposal,
            approved_proposals: proposals.iter().map(|(_, (_, p, _))| p.clone()).collect(),
        })
    }

    pub fn completed_proposal_exists(&self, completed_proposal_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT EXISTS(SELECT 1 FROM completed_proposals WHERE completed_proposal_id = ? LIMIT 1);")?;
        let mut rows = stmt.query([completed_proposal_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_completed_proposal(
        &self,
        completed_proposal_id: EventId,
        policy_id: EventId,
        completed_proposal: CompletedProposal,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO completed_proposals (completed_proposal_id, policy_id, completed_proposal) VALUES (?, ?, ?);",
            (completed_proposal_id.to_hex(), policy_id.to_hex(), completed_proposal.encrypt_with_keys(&self.keys)?),
        )?;
        log::info!("Completed proposal {completed_proposal_id} saved");
        Ok(())
    }

    pub fn completed_proposals(
        &self,
    ) -> Result<BTreeMap<EventId, (EventId, CompletedProposal)>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT completed_proposal_id, policy_id, completed_proposal FROM completed_proposals;",
        )?;
        let mut rows = stmt.query([])?;
        let mut proposals = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let proposal_id: String = row.get(0)?;
            let policy_id: String = row.get(1)?;
            let proposal: String = row.get(2)?;
            proposals.insert(
                EventId::from_hex(proposal_id)?,
                (
                    EventId::from_hex(policy_id)?,
                    CompletedProposal::decrypt_with_keys(&self.keys, proposal)?,
                ),
            );
        }
        Ok(proposals)
    }

    pub fn get_completed_proposal(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<(EventId, CompletedProposal), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT policy_id, completed_proposal FROM completed_proposals WHERE completed_proposal_id = ? LIMIT 1;")?;
        let mut rows = stmt.query([completed_proposal_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound)?;
        let policy_id: String = row.get(0)?;
        let proposal: String = row.get(1)?;
        Ok((
            EventId::from_hex(policy_id)?,
            CompletedProposal::decrypt_with_keys(&self.keys, proposal)?,
        ))
    }

    pub fn delete_completed_proposal(&self, completed_proposal_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM completed_proposals WHERE completed_proposal_id = ?;",
            [completed_proposal_id.to_hex()],
        )?;
        log::info!("Deleted completed proposal {completed_proposal_id}");
        Ok(())
    }

    fn get_description_by_txid(&self, txid: Txid) -> Result<Option<String>, Error> {
        for (_, (_, proposal)) in self.completed_proposals()?.into_iter() {
            if let CompletedProposal::Spending {
                tx, description, ..
            } = proposal
            {
                if tx.txid() == txid {
                    return Ok(Some(description));
                }
            }
        }
        Ok(None)
    }

    pub fn get_txs_descriptions(&self) -> Result<HashMap<Txid, String>, Error> {
        let mut map = HashMap::new();
        for (_, (_, proposal)) in self.completed_proposals()?.into_iter() {
            if let CompletedProposal::Spending {
                tx, description, ..
            } = proposal
            {
                if let HashMapEntry::Vacant(e) = map.entry(tx.txid()) {
                    e.insert(description);
                }
            }
        }
        Ok(map)
    }

    pub fn get_balance(&self, policy_id: EventId) -> Option<Balance> {
        let wallets = self.wallets.lock();
        let wallet = wallets.get(&policy_id)?;
        wallet.get_balance().ok()
    }

    pub fn get_transactions(&self, policy_id: EventId) -> Option<Transactions> {
        let wallets = self.wallets.lock();
        let descriptions = self.get_txs_descriptions().ok()?;
        let wallet = wallets.get(&policy_id)?;
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
        let wallet = wallets.get(&policy_id)?;
        wallet
            .get_address(AddressIndex::LastUnused)
            .ok()
            .map(|a| a.address)
    }

    pub fn get_total_balance(&self) -> Result<(Balance, bool), Error> {
        let mut synced = true;
        let mut total_balance = Balance::default();
        let mut already_seen = Vec::new();
        for (policy_id, GetPolicyResult { policy, last_sync }) in self.get_policies()?.into_iter() {
            if !already_seen.contains(&policy.descriptor) {
                if last_sync.is_none() {
                    synced = false;
                    break;
                }
                let balance = self.get_balance(policy_id).unwrap_or_default();
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
        for (policy_id, wallet) in wallets.iter() {
            let GetPolicyResult { policy, .. } = self.get_policy(*policy_id)?;
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
        for (policy_id, wallet) in wallets.iter() {
            let GetPolicyResult { policy, .. } = self.get_policy(*policy_id).ok()?;
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

    pub fn schedule_for_sync(&self, policy_id: EventId) -> Result<(), Error> {
        self.update_last_sync(policy_id, None)
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

        for (policy_id, GetPolicyResult { policy, last_sync }) in self.get_policies()?.into_iter() {
            let last_sync: Timestamp = last_sync.unwrap_or_else(|| Timestamp::from(0));
            if force || last_sync.add(WALLET_SYNC_INTERVAL) <= Timestamp::now() {
                log::info!("Syncing policy {policy_id}");
                let db: SqliteDatabase = self.get_wallet_db(policy_id)?;
                let wallet = Wallet::new(&policy.descriptor.to_string(), None, self.network, db)?;
                wallet.sync(blockchain, SyncOptions::default())?;
                self.update_last_sync(policy_id, Some(Timestamp::now()))?;
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

    pub fn save_last_relay_sync(&self, relay_url: &Url, timestamp: Timestamp) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let last_sync: u64 = timestamp.as_u64();
        let mut stmt = conn.prepare_cached("INSERT INTO relays (url, last_sync) VALUES (?, ?) ON CONFLICT(url) DO UPDATE SET last_sync = ?;")?;
        stmt.execute((relay_url.to_string(), last_sync, last_sync))?;
        Ok(())
    }

    pub fn get_last_relay_sync(&self, relay_url: &Url) -> Result<Timestamp, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT last_sync FROM relays WHERE url = ?")?;
        let mut rows = stmt.query([relay_url.to_string()])?;
        let row = rows.next()?.ok_or(Error::NotFound)?;
        let last_sync: Option<u64> = row.get(0)?;
        let last_sync: u64 = last_sync.unwrap_or_default();
        Ok(Timestamp::from(last_sync))
    }
}
