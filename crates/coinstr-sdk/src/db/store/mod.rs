// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//! Store

#![allow(clippy::type_complexity)]

use std::collections::hash_map::Entry;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use bdk::bitcoin::{Address, Network, Txid};
use bdk::blockchain::Blockchain;
use bdk::database::SqliteDatabase;
use bdk::miniscript::{Descriptor, DescriptorPublicKey};
use bdk::wallet::AddressIndex;
use bdk::{Balance, LocalUtxo, SyncOptions, TransactionDetails, Wallet};
use coinstr_core::policy::Policy;
use coinstr_core::proposal::{CompletedProposal, Proposal};
use coinstr_core::signer::{SharedSigner, Signer};
use coinstr_core::util::serde::Serde;
use coinstr_core::ApprovedProposal;
use nostr_sdk::event::id::EventId;
use nostr_sdk::nips::nip46::{Message as NIP46Message, NostrConnectURI};
use nostr_sdk::secp256k1::{SecretKey, XOnlyPublicKey};
use nostr_sdk::{Event, Keys, Metadata, Timestamp, Url};
use parking_lot::Mutex;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::config::DbConfig;
use rusqlite::OpenFlags;
use tokio::sync::broadcast::Sender;

mod relays;

use super::migration::{self, STARTUP_SQL};
use super::model::{
    GetApprovedProposalResult, GetApprovedProposals, GetDetailedPolicyResult,
    GetNotificationsResult, GetPolicyResult, GetSharedSignerResult, NostrConnectRequest,
};
use super::Error;
use crate::client::Message;
use crate::constants::{BLOCK_HEIGHT_SYNC_INTERVAL, METADATA_SYNC_INTERVAL, WALLET_SYNC_INTERVAL};
use crate::types::Notification;
use crate::util;
use crate::util::encryption::EncryptionWithKeys;

pub(crate) type SqlitePool = r2d2::Pool<SqliteConnectionManager>;
pub(crate) type PooledConnection = r2d2::PooledConnection<SqliteConnectionManager>;
pub type Transactions = Vec<(TransactionDetails, Option<String>)>;

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
    block_height: BlockHeight,
    wallets: Arc<Mutex<HashMap<EventId, Wallet<SqliteDatabase>>>>,
    nostr_connect_auto_approve: Arc<Mutex<HashMap<XOnlyPublicKey, Timestamp>>>,
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
        Ok(Self {
            pool,
            keys: keys.clone(),
            wallets: Arc::new(Mutex::new(HashMap::new())),
            nostr_connect_auto_approve: Arc::new(Mutex::new(HashMap::new())),
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

    pub fn wipe(&self) -> Result<(), Error> {
        let mut conn = self.pool.get()?;

        // Reset DB
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)?;
        conn.execute("VACUUM;", [])?;
        conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false)?;

        // Execute migrations
        conn.execute_batch(STARTUP_SQL)?;
        migration::run(&mut conn)?;

        Ok(())
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
        let row = rows.next()?.ok_or(Error::NotFound("policy".into()))?;
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
        let row = rows.next()?.ok_or(Error::NotFound("policy".into()))?;
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
        let proposal_ids = self.get_proposal_ids_by_policy_id(policy_id)?;
        for proposal_id in proposal_ids.into_iter() {
            self.delete_proposal(proposal_id)?;
        }

        let completed_proposal_ids = self.get_completed_proposal_ids_by_policy_id(policy_id)?;
        for completed_proposal_id in completed_proposal_ids.into_iter() {
            self.delete_completed_proposal(completed_proposal_id)?;
        }

        self.set_event_as_deleted(policy_id)?;

        // Delete notification
        self.delete_notification(policy_id)?;

        // Delete policy
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
        log::info!("Deleted policy {policy_id}");
        Ok(())
    }

    pub fn get_event_ids_linked_to_policy(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<EventId>, Error> {
        let proposal_ids = self.get_proposal_ids_by_policy_id(policy_id)?;
        let completed_proposal_ids = self.get_completed_proposal_ids_by_policy_id(policy_id)?;
        Ok([proposal_ids, completed_proposal_ids].concat())
    }

    pub fn shared_key_exists_for_policy(&self, policy_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT EXISTS(SELECT 1 FROM shared_keys WHERE policy_id = ? LIMIT 1);")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
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
        let row = rows.next()?.ok_or(Error::NotFound("shared_key".into()))?;
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
        let balance = self.get_balance(policy_id);
        let list = self.get_txs_with_descriptions(policy_id);
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

    fn get_proposal_ids_by_policy_id(&self, policy_id: EventId) -> Result<Vec<EventId>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT proposal_id FROM proposals WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut ids = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let proposal_id: String = row.get(0)?;
            ids.push(EventId::from_hex(proposal_id)?);
        }
        Ok(ids)
    }

    pub fn get_proposals_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<BTreeMap<EventId, (EventId, Proposal)>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("SELECT proposal_id, proposal FROM proposals WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut proposals = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let proposal_id: String = row.get(0)?;
            let proposal: String = row.get(1)?;
            proposals.insert(
                EventId::from_hex(proposal_id)?,
                (
                    policy_id,
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
        let row = rows.next()?.ok_or(Error::NotFound("proposal".into()))?;
        let policy_id: String = row.get(0)?;
        let proposal: String = row.get(1)?;
        Ok((
            EventId::from_hex(policy_id)?,
            Proposal::decrypt_with_keys(&self.keys, proposal)?,
        ))
    }

    pub fn delete_proposal(&self, proposal_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(proposal_id)?;

        // Delete notification
        self.delete_notification(proposal_id)?;

        // Delete proposal
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM proposals WHERE proposal_id = ?;",
            [proposal_id.to_hex()],
        )?;

        // Delete approvals
        conn.execute(
            "DELETE FROM approved_proposals WHERE proposal_id = ?;",
            [proposal_id.to_hex()],
        )?;

        log::info!("Deleted proposal {proposal_id}");
        Ok(())
    }

    pub fn get_approvals_by_proposal_id(
        &self,
        proposal_id: EventId,
    ) -> Result<BTreeMap<EventId, GetApprovedProposalResult>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT approval_id, public_key, approved_proposal, timestamp FROM approved_proposals WHERE proposal_id = ?;")?;
        let mut rows = stmt.query([proposal_id.to_hex()])?;
        let mut approvals = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let approval_id: String = row.get(0)?;
            let public_key: String = row.get(1)?;
            let json: String = row.get(2)?;
            let timestamp: u64 = row.get(3)?;
            approvals.insert(
                EventId::from_hex(approval_id)?,
                GetApprovedProposalResult {
                    public_key: XOnlyPublicKey::from_str(&public_key)?,
                    approved_proposal: ApprovedProposal::decrypt_with_keys(&self.keys, json)?,
                    timestamp: Timestamp::from(timestamp),
                },
            );
        }
        Ok(approvals)
    }

    pub fn save_approved_proposal(
        &self,
        proposal_id: EventId,
        author: XOnlyPublicKey,
        approval_id: EventId,
        approved_proposal: ApprovedProposal,
        timestamp: Timestamp,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO approved_proposals (approval_id, proposal_id, public_key, approved_proposal, timestamp) VALUES (?, ?, ?, ?, ?);",
            (approval_id.to_hex(), proposal_id.to_hex(), author.to_string(), approved_proposal.encrypt_with_keys(&self.keys)?, timestamp.as_u64()),
        )?;
        Ok(())
    }

    pub fn get_approved_proposals_by_id(
        &self,
        proposal_id: EventId,
    ) -> Result<GetApprovedProposals, Error> {
        let (policy_id, proposal) = self.get_proposal(proposal_id)?;
        let approved_proposals = self.get_approvals_by_proposal_id(proposal_id)?;
        Ok(GetApprovedProposals {
            policy_id,
            proposal,
            approved_proposals: approved_proposals
                .iter()
                .map(
                    |(
                        _,
                        GetApprovedProposalResult {
                            approved_proposal, ..
                        },
                    )| approved_proposal.clone(),
                )
                .collect(),
        })
    }

    pub fn get_policy_id_by_approval_id(&self, approval_id: EventId) -> Result<EventId, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT proposal_id FROM approved_proposals WHERE approval_id = ? LIMIT 1;",
        )?;
        let mut rows = stmt.query([approval_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("approval".into()))?;
        let proposal_id: String = row.get(0)?;
        let proposal_id = EventId::from_hex(proposal_id)?;
        let (policy_id, ..) = self.get_proposal(proposal_id)?;
        Ok(policy_id)
    }

    pub fn approved_proposal_exists(&self, approved_proposal_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM approved_proposals WHERE approval_id = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([approved_proposal_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn delete_approval(&self, approval_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(approval_id)?;

        // Delete notification
        self.delete_notification(approval_id)?;

        // Delete policy
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM approved_proposals WHERE approval_id = ?;",
            [approval_id.to_hex()],
        )?;
        log::info!("Deleted approval {approval_id}");
        Ok(())
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
        let row = rows
            .next()?
            .ok_or(Error::NotFound("completed proposal".into()))?;
        let policy_id: String = row.get(0)?;
        let proposal: String = row.get(1)?;
        Ok((
            EventId::from_hex(policy_id)?,
            CompletedProposal::decrypt_with_keys(&self.keys, proposal)?,
        ))
    }

    fn get_completed_proposal_ids_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<EventId>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT completed_proposal_id FROM completed_proposals WHERE policy_id = ?;",
        )?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut ids = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let completed_proposal_id: String = row.get(0)?;
            ids.push(EventId::from_hex(completed_proposal_id)?);
        }
        Ok(ids)
    }

    pub fn delete_completed_proposal(&self, completed_proposal_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(completed_proposal_id)?;

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
                if let Entry::Vacant(e) = map.entry(tx.txid()) {
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

    pub fn get_txs(&self, policy_id: EventId) -> Option<Vec<TransactionDetails>> {
        let wallets = self.wallets.lock();
        let wallet = wallets.get(&policy_id)?;
        wallet.list_transactions(true).ok()
    }

    /// Get transactions with descriptions
    pub fn get_txs_with_descriptions(&self, policy_id: EventId) -> Option<Transactions> {
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

    pub fn get_utxos(&self, policy_id: EventId) -> Result<Vec<LocalUtxo>, Error> {
        let wallets = self.wallets.lock();
        let wallet = wallets.get(&policy_id).ok_or(Error::WalletNotFound)?;
        Ok(wallet.list_unspent()?)
    }

    pub fn get_total_balance(&self) -> Result<Balance, Error> {
        let mut total_balance = Balance::default();
        let mut already_seen = Vec::new();
        for (policy_id, GetPolicyResult { policy, .. }) in self.get_policies()?.into_iter() {
            if !already_seen.contains(&policy.descriptor) {
                let balance = self.get_balance(policy_id).unwrap_or_default();
                total_balance = total_balance.add(balance);
                already_seen.push(policy.descriptor);
            }
        }
        Ok(total_balance)
    }

    pub fn get_all_transactions(&self) -> Result<Vec<(TransactionDetails, Option<String>)>, Error> {
        let descriptions = self.get_txs_descriptions()?;
        let mut transactions = Vec::new();
        let mut already_seen = Vec::new();
        for (policy_id, GetPolicyResult { policy, .. }) in self.get_policies()?.into_iter() {
            if !already_seen.contains(&policy.descriptor) {
                for tx in self.get_txs(policy_id).unwrap_or_default().into_iter() {
                    let desc: Option<String> = descriptions.get(&tx.txid).cloned();
                    transactions.push((tx, desc))
                }
                already_seen.push(policy.descriptor);
            }
        }
        Ok(transactions)
    }

    pub fn get_tx(&self, txid: Txid) -> Option<(TransactionDetails, Option<String>)> {
        let desc = self.get_description_by_txid(txid).ok()?;
        let mut already_seen = Vec::new();
        for (policy_id, GetPolicyResult { policy, .. }) in self.get_policies().ok()?.into_iter() {
            if !already_seen.contains(&policy.descriptor) {
                let txs = self.get_txs(policy_id)?;
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
        sender: Option<&Sender<Option<Message>>>,
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

        let loaded_wallet_ids: Vec<EventId> = {
            let wallets = self.wallets.lock();
            wallets.keys().copied().collect()
        };

        for (policy_id, GetPolicyResult { policy, last_sync }) in self.get_policies()?.into_iter() {
            let last_sync: Timestamp = last_sync.unwrap_or_else(|| Timestamp::from(0));
            if force || last_sync.add(WALLET_SYNC_INTERVAL) <= Timestamp::now() {
                log::info!("Syncing policy {policy_id}");
                let db: SqliteDatabase = self.get_wallet_db(policy_id)?;
                let wallet = Wallet::new(&policy.descriptor.to_string(), None, self.network, db)?;
                wallet.sync(blockchain, SyncOptions::default())?;
                self.update_last_sync(policy_id, Some(Timestamp::now()))?;

                if !loaded_wallet_ids.contains(&policy_id) {
                    // Load wallet
                    let mut wallets = self.wallets.lock();
                    if let Entry::Vacant(e) = wallets.entry(policy_id) {
                        e.insert(wallet);
                    }
                }

                if let Some(sender) = sender {
                    let _ = sender.send(Some(Message::WalletSyncCompleted(policy_id)));
                }
                log::info!("Policy {policy_id} synced");
            }
        }
        Ok(())
    }

    pub fn delete_generic_event_id(&self, event_id: EventId) -> Result<(), Error> {
        if self.policy_exists(event_id)? {
            self.delete_policy(event_id)?;
        } else if self.proposal_exists(event_id)? {
            self.delete_proposal(event_id)?;
        } else if self.approved_proposal_exists(event_id)? {
            self.delete_approval(event_id)?;
        } else if self.completed_proposal_exists(event_id)? {
            self.delete_completed_proposal(event_id)?;
        } else if self.signer_exists(event_id)? {
            self.delete_signer(event_id)?;
        } else if self.my_shared_signer_exists(event_id)? || self.shared_signer_exists(event_id)? {
            self.delete_shared_signer(event_id)?;
        } else {
            self.set_event_as_deleted(event_id)?;
        };

        Ok(())
    }

    pub fn save_event(&self, event: &Event) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("INSERT OR IGNORE INTO events (event_id, event) VALUES (?, ?);")?;
        stmt.execute((event.id.to_hex(), event.as_json()))?;
        Ok(())
    }

    pub fn get_events(&self) -> Result<Vec<Event>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT event FROM events;")?;
        let mut rows = stmt.query([])?;
        let mut events: Vec<Event> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let json: String = row.get(0)?;
            let event: Event = Event::from_json(json)?;
            events.push(event);
        }
        Ok(events)
    }

    pub fn get_event_by_id(&self, event_id: EventId) -> Result<Event, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT event FROM events WHERE event_id = ?;")?;
        let mut rows = stmt.query([event_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("event".into()))?;
        let json: String = row.get(0)?;
        Ok(Event::from_json(json)?)
    }

    pub fn event_was_deleted(&self, event_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM events WHERE event_id = ? AND deleted = 1 LIMIT 1);",
        )?;
        let mut rows = stmt.query([event_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn set_event_as_deleted(&self, event_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("UPDATE events SET deleted = 1 WHERE event_id = ?")?;
        stmt.execute([event_id.to_hex()])?;
        Ok(())
    }

    pub fn save_pending_event(&self, event: &Event) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("INSERT OR IGNORE INTO pending_events (event) VALUES (?);")?;
        stmt.execute([event.as_json()])?;
        log::info!("Saved pending event {} (kind={:?})", event.id, event.kind);
        Ok(())
    }

    pub fn get_pending_events(&self) -> Result<Vec<Event>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT event FROM pending_events;")?;
        let mut rows = stmt.query([])?;
        let mut events: Vec<Event> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let json: String = row.get(0)?;
            let event: Event = Event::from_json(json)?;
            events.push(event);
        }
        Ok(events)
    }

    pub fn save_notification(
        &self,
        event_id: EventId,
        notification: Notification,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached(
            "INSERT OR IGNORE INTO notifications (event_id, notification, timestamp) VALUES (?, ?, ?);",
        )?;
        stmt.execute((
            event_id.to_hex(),
            notification.as_json(),
            Timestamp::now().as_u64(),
        ))?;
        Ok(())
    }

    pub fn get_notifications(&self) -> Result<Vec<GetNotificationsResult>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT notification, timestamp, seen FROM notifications ORDER BY timestamp DESC;",
        )?;
        let mut rows = stmt.query([])?;
        let mut notifications: Vec<GetNotificationsResult> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let json: String = row.get(0)?;
            let notification: Notification = Notification::from_json(json)?;
            let timestamp: u64 = row.get(1)?;
            let timestamp = Timestamp::from(timestamp);
            let seen: bool = row.get(2)?;
            notifications.push(GetNotificationsResult {
                notification,
                timestamp,
                seen,
            });
        }
        Ok(notifications)
    }

    pub fn count_unseen_notifications(&self) -> Result<usize, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT COUNT(*) FROM notifications WHERE seen = 0;")?;
        let mut rows = stmt.query([])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("count notifications".into()))?;
        let count: usize = row.get(0)?;
        Ok(count)
    }

    pub fn mark_notification_as_seen_by_id(&self, event_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("UPDATE notifications SET seen = 1 WHERE event_id = ?")?;
        stmt.execute([event_id.to_hex()])?;
        Ok(())
    }

    pub fn mark_notification_as_seen(&self, notification: Notification) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("UPDATE notifications SET seen = 1 WHERE notification = ?")?;
        stmt.execute([notification.as_json()])?;
        Ok(())
    }

    pub fn mark_all_notifications_as_seen(&self) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("UPDATE notifications SET seen = 1;")?;
        stmt.execute([])?;
        Ok(())
    }

    pub fn delete_all_notifications(&self) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("DELETE FROM notifications;")?;
        stmt.execute([])?;
        Ok(())
    }

    pub fn delete_notification(&self, event_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("DELETE FROM notifications WHERE event_id = ?")?;
        stmt.execute([event_id.to_hex()])?;
        Ok(())
    }

    pub fn signer_exists(&self, signer_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT EXISTS(SELECT 1 FROM signers WHERE signer_id = ? LIMIT 1);")?;
        let mut rows = stmt.query([signer_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_signer(&self, signer_id: EventId, signer: Signer) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("INSERT OR IGNORE INTO signers (signer_id, signer) VALUES (?, ?);")?;
        stmt.execute((signer_id.to_hex(), signer.encrypt_with_keys(&self.keys)?))?;
        log::info!("Saved signer {signer_id}");
        Ok(())
    }

    pub(crate) fn get_signers(&self) -> Result<BTreeMap<EventId, Signer>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT signer_id, signer FROM signers;")?;
        let mut rows = stmt.query([])?;
        let mut signers = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let signer_id: String = row.get(0)?;
            let signer: String = row.get(1)?;
            signers.insert(
                EventId::from_hex(signer_id)?,
                Signer::decrypt_with_keys(&self.keys, signer)?,
            );
        }
        Ok(signers)
    }

    pub(crate) fn signer_descriptor_exists(
        &self,
        descriptor: Descriptor<DescriptorPublicKey>,
    ) -> Result<bool, Error> {
        let signers = self.get_signers()?;
        for signer in signers.into_values() {
            if signer.descriptor() == descriptor {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn get_signer_by_id(&self, signer_id: EventId) -> Result<Signer, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT signer FROM signers WHERE signer_id = ?;")?;
        let mut rows = stmt.query([signer_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("signer".into()))?;
        let signer: String = row.get(0)?;
        Ok(Signer::decrypt_with_keys(&self.keys, signer)?)
    }

    pub fn delete_signer(&self, signer_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(signer_id)?;

        // Delete notification
        //self.delete_notification(Notification::NewProposal(proposal_id))?;

        // Delete signer
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM signers WHERE signer_id = ?;",
            [signer_id.to_hex()],
        )?;

        conn.execute(
            "DELETE FROM my_shared_signers WHERE signer_id = ?;",
            [signer_id.to_hex()],
        )?;

        log::info!("Deleted signer {signer_id}");
        Ok(())
    }

    pub fn delete_shared_signer(&self, shared_signer_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(shared_signer_id)?;

        // Delete notification
        self.delete_notification(shared_signer_id)?;

        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM my_shared_signers WHERE shared_signer_id = ?;",
            [shared_signer_id.to_hex()],
        )?;
        conn.execute(
            "DELETE FROM shared_signers WHERE shared_signer_id = ?;",
            [shared_signer_id.to_hex()],
        )?;
        log::info!("Deleted shared signer {shared_signer_id}");
        Ok(())
    }

    pub fn my_shared_signer_exists(&self, shared_signer_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM my_shared_signers WHERE shared_signer_id = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([shared_signer_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn shared_signer_exists(&self, shared_signer_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM shared_signers WHERE shared_signer_id = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([shared_signer_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn my_shared_signer_already_shared(
        &self,
        signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM my_shared_signers WHERE signer_id = ? AND public_key = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([signer_id.to_hex(), public_key.to_string()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_my_shared_signer(
        &self,
        signer_id: EventId,
        shared_signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("INSERT OR IGNORE INTO my_shared_signers (signer_id, shared_signer_id, public_key) VALUES (?, ?, ?);")?;
        stmt.execute((
            signer_id.to_hex(),
            shared_signer_id.to_hex(),
            public_key.to_string(),
        ))?;
        log::info!("Saved my shared signer {shared_signer_id} (signer {signer_id})");
        Ok(())
    }

    pub fn save_shared_signer(
        &self,
        shared_signer_id: EventId,
        owner_public_key: XOnlyPublicKey,
        shared_signer: SharedSigner,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("INSERT OR IGNORE INTO shared_signers (shared_signer_id, owner_public_key, shared_signer) VALUES (?, ?, ?);")?;
        stmt.execute((
            shared_signer_id.to_hex(),
            owner_public_key.to_string(),
            shared_signer.encrypt_with_keys(&self.keys)?,
        ))?;
        log::info!("Saved shared signer {shared_signer_id}");
        Ok(())
    }

    pub fn get_public_key_for_my_shared_signer(
        &self,
        shared_signer_id: EventId,
    ) -> Result<XOnlyPublicKey, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT public_key FROM my_shared_signers WHERE shared_signer_id = ? LIMIT 1;",
        )?;
        let mut rows = stmt.query([shared_signer_id.to_hex()])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("my shared signer".into()))?;
        let public_key: String = row.get(0)?;
        Ok(XOnlyPublicKey::from_str(&public_key)?)
    }

    pub fn get_my_shared_signers(&self) -> Result<BTreeMap<EventId, XOnlyPublicKey>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT shared_signer_id, public_key FROM my_shared_signers;")?;
        let mut rows = stmt.query([])?;
        let mut map = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let shared_signer_id: String = row.get(0)?;
            let public_key: String = row.get(1)?;
            map.insert(
                EventId::from_hex(shared_signer_id)?,
                XOnlyPublicKey::from_str(&public_key)?,
            );
        }
        Ok(map)
    }

    pub fn get_my_shared_signers_by_signer_id(
        &self,
        signer_id: EventId,
    ) -> Result<BTreeMap<EventId, XOnlyPublicKey>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT shared_signer_id, public_key FROM my_shared_signers WHERE signer_id = ?;",
        )?;
        let mut rows = stmt.query([signer_id.to_hex()])?;
        let mut map = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let shared_signer_id: String = row.get(0)?;
            let public_key: String = row.get(1)?;
            map.insert(
                EventId::from_hex(shared_signer_id)?,
                XOnlyPublicKey::from_str(&public_key)?,
            );
        }
        Ok(map)
    }

    pub fn get_owner_public_key_for_shared_signer(
        &self,
        shared_signer_id: EventId,
    ) -> Result<XOnlyPublicKey, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT owner_public_key FROM shared_signers WHERE shared_signer_id = ? LIMIT 1;",
        )?;
        let mut rows = stmt.query([shared_signer_id.to_hex()])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("shared signer".into()))?;
        let public_key: String = row.get(0)?;
        Ok(XOnlyPublicKey::from_str(&public_key)?)
    }

    pub fn get_shared_signers(&self) -> Result<BTreeMap<EventId, GetSharedSignerResult>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT shared_signer_id, owner_public_key, shared_signer FROM shared_signers;",
        )?;
        let mut rows = stmt.query([])?;
        let mut map = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let shared_signer_id: String = row.get(0)?;
            let public_key: String = row.get(1)?;
            let shared_signer: String = row.get(2)?;
            map.insert(
                EventId::from_hex(shared_signer_id)?,
                GetSharedSignerResult {
                    owner_public_key: XOnlyPublicKey::from_str(&public_key)?,
                    shared_signer: SharedSigner::decrypt_with_keys(&self.keys, shared_signer)?,
                },
            );
        }
        Ok(map)
    }

    pub fn get_contacts_public_keys(&self) -> Result<HashSet<XOnlyPublicKey>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT public_key FROM contacts;")?;
        let mut rows = stmt.query([])?;
        let mut public_keys = HashSet::new();
        while let Ok(Some(row)) = rows.next() {
            let public_key: String = row.get(0)?;
            public_keys.insert(XOnlyPublicKey::from_str(&public_key)?);
        }
        Ok(public_keys)
    }

    pub fn delete_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM contacts WHERE public_key = ?;",
            [public_key.to_string()],
        )?;
        log::info!("Deleted contact {public_key}");
        Ok(())
    }

    pub fn save_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        if public_key != self.keys.public_key() {
            let conn = self.pool.get()?;
            let mut stmt =
                conn.prepare_cached("INSERT OR IGNORE INTO contacts (public_key) VALUES (?);")?;
            stmt.execute([public_key.to_string()])?;
            let mut stmt = conn.prepare_cached(
                "INSERT OR IGNORE INTO metadata (public_key, metadata) VALUES (?, ?);",
            )?;
            stmt.execute([public_key.to_string(), Metadata::default().as_json()])?;
            log::info!("Saved contact {public_key}");
        }

        Ok(())
    }

    pub(crate) fn save_contacts(&self, contacts: HashSet<XOnlyPublicKey>) -> Result<(), Error> {
        let public_keys = self.get_contacts_public_keys()?;

        for pk in public_keys.difference(&contacts) {
            self.delete_contact(*pk)?;
        }

        for public_key in contacts.difference(&public_keys) {
            self.save_contact(*public_key)?;
        }

        Ok(())
    }

    fn get_metadata_with_conn(
        &self,
        conn: &PooledConnection,
        public_key: XOnlyPublicKey,
    ) -> Result<Metadata, Error> {
        let mut stmt = conn.prepare("SELECT metadata FROM metadata WHERE public_key = ?;")?;
        let mut rows = stmt.query([public_key.to_string()])?;
        match rows.next()? {
            Some(row) => {
                let metadata: String = row.get(0)?;
                Ok(Metadata::from_json(metadata)?)
            }
            None => {
                // Save public_key to metadata table
                let metadata = Metadata::default();
                conn.execute(
                    "INSERT OR IGNORE INTO metadata (public_key, metadata) VALUES (?, ?);",
                    (public_key.to_string(), metadata.as_json()),
                )?;
                Ok(metadata)
            }
        }
    }

    pub fn get_metadata(&self, public_key: XOnlyPublicKey) -> Result<Metadata, Error> {
        let conn = self.pool.get()?;
        self.get_metadata_with_conn(&conn, public_key)
    }

    pub fn get_public_key_name(&self, public_key: XOnlyPublicKey) -> String {
        match self.get_metadata(public_key) {
            Ok(metadata) => {
                if let Some(display_name) = metadata.display_name {
                    display_name
                } else if let Some(name) = metadata.name {
                    name
                } else {
                    util::cut_public_key(public_key)
                }
            }
            Err(e) => {
                log::error!("{e}");
                util::cut_public_key(public_key)
            }
        }
    }

    pub fn get_contacts_with_metadata(&self) -> Result<BTreeMap<XOnlyPublicKey, Metadata>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT public_key FROM contacts;")?;
        let mut rows = stmt.query([])?;
        let mut contacts = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let public_key: String = row.get(0)?;
            let public_key = XOnlyPublicKey::from_str(&public_key)?;
            contacts.insert(public_key, self.get_metadata_with_conn(&conn, public_key)?);
        }
        Ok(contacts)
    }

    pub fn get_unsynced_metadata_pubkeys(&self) -> Result<Vec<XOnlyPublicKey>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT public_key, last_sync FROM metadata;")?;
        let mut rows = stmt.query([])?;
        let mut public_keys: Vec<XOnlyPublicKey> = Vec::new();
        let now = Timestamp::now();
        while let Ok(Some(row)) = rows.next() {
            let public_key: String = row.get(0)?;
            let public_key: XOnlyPublicKey = XOnlyPublicKey::from_str(&public_key)?;
            let last_sync: Option<u64> = row.get(1)?;

            if let Some(last_sync) = last_sync {
                if last_sync + METADATA_SYNC_INTERVAL.as_secs() < now.as_u64() {
                    public_keys.push(public_key);
                }
            } else {
                public_keys.push(public_key);
            }
        }
        Ok(public_keys)
    }

    pub fn set_metadata(
        &self,
        public_key: XOnlyPublicKey,
        metadata: Metadata,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let last_sync = Timestamp::now().as_u64();
        let mut stmt = conn.prepare_cached("INSERT INTO metadata (public_key, metadata, last_sync) VALUES (?, ?, ?) ON CONFLICT(public_key) DO UPDATE SET metadata = ?, last_sync = ?;")?;
        stmt.execute((
            public_key.to_string(),
            metadata.as_json(),
            last_sync,
            metadata.as_json(),
            last_sync,
        ))?;
        Ok(())
    }

    pub fn save_nostr_connect_uri(&self, uri: NostrConnectURI) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO nostr_connect_sessions (app_public_key, uri, timestamp) VALUES (?, ?, ?);",
            (uri.public_key.to_string(), uri.to_string(), Timestamp::now().as_u64()),
        )?;
        Ok(())
    }

    pub fn nostr_connect_session_exists(
        &self,
        app_public_key: XOnlyPublicKey,
    ) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM nostr_connect_sessions WHERE app_public_key = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([app_public_key.to_string()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_nostr_connect_request(
        &self,
        event_id: EventId,
        app_public_key: XOnlyPublicKey,
        message: NIP46Message,
        timestamp: Timestamp,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO nostr_connect_requests (event_id, app_public_key, message, timestamp) VALUES (?, ?, ?, ?);",
            (event_id.to_hex(), app_public_key.to_string(), message.as_json(), timestamp.as_u64()),
        )?;
        Ok(())
    }

    pub fn get_nostr_connect_sessions(&self) -> Result<Vec<(NostrConnectURI, Timestamp)>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT uri, timestamp FROM nostr_connect_sessions;")?;
        let mut rows = stmt.query([])?;
        let mut sessions: Vec<(NostrConnectURI, Timestamp)> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let uri: String = row.get(0)?;
            let uri: NostrConnectURI = NostrConnectURI::from_str(&uri)?;
            let timestamp: u64 = row.get(1)?;
            sessions.push((uri, Timestamp::from(timestamp)));
        }
        Ok(sessions)
    }

    pub fn get_nostr_connect_sessions_relays(&self) -> Result<Vec<Url>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT uri FROM nostr_connect_sessions;")?;
        let mut rows = stmt.query([])?;
        let mut urls = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let uri: String = row.get(0)?;
            let uri: NostrConnectURI = NostrConnectURI::from_str(&uri)?;
            if !urls.contains(&uri.relay_url) {
                urls.push(uri.relay_url);
            }
        }
        Ok(urls)
    }

    pub fn get_nostr_connect_session(
        &self,
        app_public_key: XOnlyPublicKey,
    ) -> Result<NostrConnectURI, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT uri FROM nostr_connect_sessions WHERE app_public_key = ?;")?;
        let mut rows = stmt.query([app_public_key.to_string()])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("nostr connect session".into()))?;
        let uri: String = row.get(0)?;
        Ok(NostrConnectURI::from_str(&uri)?)
    }

    pub fn delete_nostr_connect_session(
        &self,
        app_public_key: XOnlyPublicKey,
    ) -> Result<(), Error> {
        // Delete notifications
        //self.delete_notification(policy_id)?;

        // Delete session
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM nostr_connect_sessions WHERE app_public_key = ?;",
            [app_public_key.to_string()],
        )?;
        conn.execute(
            "DELETE FROM nostr_connect_requests WHERE app_public_key = ?;",
            [app_public_key.to_string()],
        )?;
        log::info!("Deleted nostr connect session {app_public_key}");
        Ok(())
    }

    pub fn get_nostr_connect_requests(
        &self,
        approved: bool,
    ) -> Result<BTreeMap<EventId, NostrConnectRequest>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT event_id, app_public_key, message, timestamp, approved FROM nostr_connect_requests WHERE approved = ?;")?;
        let mut rows = stmt.query([approved])?;
        let mut requests: BTreeMap<EventId, NostrConnectRequest> = BTreeMap::new();
        while let Ok(Some(row)) = rows.next() {
            let event_id: String = row.get(0)?;
            let app_public_key: String = row.get(1)?;
            let message: String = row.get(2)?;
            let timestamp: u64 = row.get(3)?;
            let approved: bool = row.get(4)?;
            requests.insert(
                EventId::from_hex(event_id)?,
                NostrConnectRequest {
                    app_public_key: XOnlyPublicKey::from_str(&app_public_key)?,
                    message: NIP46Message::from_json(message)?,
                    timestamp: Timestamp::from(timestamp),
                    approved,
                },
            );
        }
        Ok(requests)
    }

    pub fn get_nostr_connect_request(
        &self,
        event_id: EventId,
    ) -> Result<NostrConnectRequest, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT app_public_key, message, timestamp, approved FROM nostr_connect_requests WHERE event_id = ?;")?;
        let mut rows = stmt.query([event_id.to_hex()])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("nostr connect request".into()))?;
        let app_public_key: String = row.get(0)?;
        let message: String = row.get(1)?;
        let timestamp: u64 = row.get(2)?;
        let approved: bool = row.get(3)?;
        Ok(NostrConnectRequest {
            app_public_key: XOnlyPublicKey::from_str(&app_public_key)?,
            message: NIP46Message::from_json(message)?,
            timestamp: Timestamp::from(timestamp),
            approved,
        })
    }

    pub fn set_nostr_connect_request_as_approved(&self, event_id: EventId) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("UPDATE nostr_connect_requests SET approved = 1 WHERE event_id = ?")?;
        stmt.execute([event_id.to_hex()])?;
        Ok(())
    }

    pub fn set_nostr_connect_auto_approve(&self, app_public_key: XOnlyPublicKey, until: Timestamp) {
        let mut nostr_connect_auto_approve = self.nostr_connect_auto_approve.lock();
        nostr_connect_auto_approve.insert(app_public_key, until);
    }

    pub fn is_nostr_connect_session_pre_authorized(&self, app_public_key: XOnlyPublicKey) -> bool {
        let mut nostr_connect_auto_approve = self.nostr_connect_auto_approve.lock();
        if let Some(until) = nostr_connect_auto_approve.get(&app_public_key) {
            if Timestamp::now() < *until {
                return true;
            } else {
                nostr_connect_auto_approve.remove(&app_public_key);
            }
        }
        false
    }

    pub fn revoke_nostr_connect_auto_approve(&self, app_public_key: XOnlyPublicKey) {
        let mut nostr_connect_auto_approve = self.nostr_connect_auto_approve.lock();
        nostr_connect_auto_approve.remove(&app_public_key);
    }

    pub fn get_nostr_connect_pre_authorizations(&self) -> BTreeMap<XOnlyPublicKey, Timestamp> {
        let nostr_connect_auto_approve = self.nostr_connect_auto_approve.lock();
        nostr_connect_auto_approve
            .iter()
            .map(|(pk, ts)| (*pk, *ts))
            .collect()
    }

    pub fn delete_nostr_connect_request(&self, event_id: EventId) -> Result<(), Error> {
        // Delete notifications
        //self.delete_notification(policy_id)?;

        // Delete
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM nostr_connect_requests WHERE event_id = ?;",
            [event_id.to_hex()],
        )?;
        log::info!("Deleted nostr connect request {event_id}");
        Ok(())
    }
}
