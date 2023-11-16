// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::Policy;
use smartvaults_protocol::nostr::{EventId, Timestamp};

use crate::encryption::StoreEncryption;
use crate::model::InternalGetPolicy;
use crate::{Error, Store};

impl Store {
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn save_policy(
        &self,
        policy_id: EventId,
        policy: Policy,
        nostr_public_keys: Vec<XOnlyPublicKey>,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO policies (policy_id, policy) VALUES (?, ?);",
                (policy_id.to_hex(), policy.encrypt(&cipher)?),
            )?;
            // Save nostr public keys
            let mut stmt = conn.prepare_cached(
                "INSERT OR IGNORE INTO nostr_public_keys (policy_id, public_key) VALUES (?, ?);",
            )?;
            for public_key in nostr_public_keys.into_iter() {
                stmt.execute((policy_id.to_hex(), public_key.to_string()))?;
            }
            Ok(())
        })
        .await?
    }

    pub async fn get_policy(&self, policy_id: EventId) -> Result<InternalGetPolicy, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT policy, last_sync FROM policies WHERE policy_id = ?")?;
            let mut rows = stmt.query([policy_id.to_hex()])?;
            let row = rows.next()?.ok_or(Error::NotFound("policy".into()))?;
            let policy: Vec<u8> = row.get(0)?;
            let last_sync: Option<u64> = row.get(1)?;
            Ok(InternalGetPolicy {
                policy_id,
                policy: Policy::decrypt(&cipher, policy)?,
                last_sync: last_sync.map(Timestamp::from),
            })
        })
        .await?
    }

    pub async fn get_last_sync(&self, policy_id: EventId) -> Result<Option<Timestamp>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT last_sync FROM policies WHERE policy_id = ?")?;
            let mut rows = stmt.query([policy_id.to_hex()])?;
            let row = rows.next()?.ok_or(Error::NotFound("policy".into()))?;
            let last_sync: Option<u64> = row.get(0)?;
            Ok(last_sync.map(Timestamp::from))
        })
        .await?
    }

    pub async fn update_last_sync(
        &self,
        policy_id: EventId,
        last_sync: Option<Timestamp>,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("UPDATE policies SET last_sync = ? WHERE policy_id = ?")?;
            stmt.execute((last_sync.map(|t| t.as_u64()), policy_id.to_hex()))?;
            Ok(())
        })
        .await?
    }

    pub async fn get_policies(&self) -> Result<Vec<InternalGetPolicy>, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT policy_id, policy, last_sync FROM policies")?;
            let mut rows = stmt.query([])?;
            let mut policies = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let policy_id: String = row.get(0)?;
                let policy: Vec<u8> = row.get(1)?;
                let last_sync: Option<u64> = row.get(2)?;
                policies.push(InternalGetPolicy {
                    policy_id: EventId::from_hex(policy_id)?,
                    policy: Policy::decrypt(&cipher, policy)?,
                    last_sync: last_sync.map(Timestamp::from),
                });
            }

            policies.sort();

            Ok(policies)
        })
        .await?
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn delete_policy(&self, policy_id: EventId) -> Result<(), Error> {
        let proposal_ids = self.get_proposal_ids_by_policy_id(policy_id).await?;
        for proposal_id in proposal_ids.into_iter() {
            // TODO: use execute_batch
            self.delete_proposal(proposal_id).await?;
        }

        let completed_proposal_ids = self
            .get_completed_proposal_ids_by_policy_id(policy_id)
            .await?;
        for completed_proposal_id in completed_proposal_ids.into_iter() {
            // TODO: use execute_batch
            self.delete_completed_proposal(completed_proposal_id)
                .await?;
        }

        // Delete policy
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
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
            tracing::info!("Deleted policy {policy_id}");
            Ok(())
        })
        .await?
    }

    pub async fn get_event_ids_linked_to_policy(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<EventId>, Error> {
        let proposal_ids = self.get_proposal_ids_by_policy_id(policy_id).await?;
        let completed_proposal_ids = self
            .get_completed_proposal_ids_by_policy_id(policy_id)
            .await?;
        Ok([proposal_ids, completed_proposal_ids].concat())
    }
}
