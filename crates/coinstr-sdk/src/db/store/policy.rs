// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::hash_map::Entry;
use std::collections::BTreeMap;

use bdk::database::SqliteDatabase;
use bdk::Wallet;
use coinstr_core::Policy;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{EventId, Timestamp};

use super::Store;
use crate::db::model::{GetDetailedPolicyResult, GetPolicy};
use crate::db::Error;
use crate::util::encryption::EncryptionWithKeys;

impl Store {
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

    #[tracing::instrument(skip_all, level = "trace")]
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

    pub fn get_policy(&self, policy_id: EventId) -> Result<GetPolicy, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT policy, last_sync FROM policies WHERE policy_id = ?")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("policy".into()))?;
        let policy: String = row.get(0)?;
        let last_sync: Option<u64> = row.get(1)?;
        Ok(GetPolicy {
            policy_id,
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

    pub fn get_policies(&self) -> Result<Vec<GetPolicy>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT policy_id, policy, last_sync FROM policies")?;
        let mut rows = stmt.query([])?;
        let mut policies = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let policy_id: String = row.get(0)?;
            let policy: String = row.get(1)?;
            let last_sync: Option<u64> = row.get(2)?;
            policies.push(GetPolicy {
                policy_id: EventId::from_hex(policy_id)?,
                policy: Policy::decrypt_with_keys(&self.keys, policy)?,
                last_sync: last_sync.map(Timestamp::from),
            });
        }
        Ok(policies)
    }

    pub fn get_detailed_policies(
        &self,
    ) -> Result<BTreeMap<EventId, GetDetailedPolicyResult>, Error> {
        let mut policies = BTreeMap::new();
        for GetPolicy {
            policy_id,
            policy,
            last_sync,
        } in self.get_policies()?.into_iter()
        {
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

    #[tracing::instrument(skip_all, level = "trace")]
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
        tracing::info!("Deleted policy {policy_id}");
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
}
