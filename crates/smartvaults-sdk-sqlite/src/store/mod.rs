// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Store

#![allow(clippy::type_complexity)]

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

use chacha20poly1305::aead::KeyInit;
use chacha20poly1305::XChaCha20Poly1305;
use deadpool_sqlite::{Config, Object, Pool, Runtime};
use rusqlite::config::DbConfig;
use smartvaults_core::bitcoin::Txid;
use smartvaults_core::proposal::CompletedProposal;
use smartvaults_protocol::nostr::event::id::EventId;
use smartvaults_protocol::nostr::secp256k1::XOnlyPublicKey;
use smartvaults_protocol::nostr::{Event, JsonUtil, Keys, Timestamp};
use tokio::sync::RwLock;

mod connect;
mod label;
mod relays;
mod signers;
mod timechain;
mod utxos;

use super::encryption::StoreEncryption;
use super::migration::{self, STARTUP_SQL};
use super::model::GetCompletedProposal;
use super::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    CompletedProposal { completed_proposal_id: EventId },
    Signer { signer_id: EventId },
    MySharedSigner { my_shared_signer_id: EventId },
    SharedSigner { shared_signer_id: EventId },
}

/// Store
#[derive(Clone)]
pub struct Store {
    pool: Pool,
    cipher: XChaCha20Poly1305,
    nostr_connect_auto_approve: Arc<RwLock<HashMap<XOnlyPublicKey, Timestamp>>>,
}

impl Debug for Store {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<sensitive>")
    }
}

impl Drop for Store {
    fn drop(&mut self) {}
}

impl Store {
    /// Open new database
    pub async fn open<P>(user_db_path: P, keys: &Keys) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let cfg = Config::new(user_db_path.as_ref());
        let pool = cfg.create_pool(Runtime::Tokio1)?;
        let conn = pool.get().await?;
        migration::run(&conn).await?;
        let key: [u8; 32] = keys.secret_key()?.secret_bytes();
        Ok(Self {
            pool,
            cipher: XChaCha20Poly1305::new(&key.into()),
            nostr_connect_auto_approve: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    async fn acquire(&self) -> Result<Object, Error> {
        Ok(self.pool.get().await?)
    }

    /// Close db
    pub fn close(self) {
        drop(self);
    }

    pub async fn wipe(&self) -> Result<(), Error> {
        let conn = self.acquire().await?;

        conn.interact(|conn| {
            // Reset DB
            conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, true)?;
            conn.execute("VACUUM;", [])?;
            conn.set_db_config(DbConfig::SQLITE_DBCONFIG_RESET_DATABASE, false)?;

            // Execute migrations
            conn.execute_batch(STARTUP_SQL)?;

            Ok::<(), Error>(())
        })
        .await??;

        migration::run(&conn).await?;

        Ok(())
    }

    pub async fn exists(&self, t: Type) -> Result<bool, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let (sql, params) = match t {
                Type::CompletedProposal { completed_proposal_id } => {
                    ("SELECT EXISTS(SELECT 1 FROM completed_proposals WHERE completed_proposal_id = ? LIMIT 1);", [completed_proposal_id.to_hex()])
                },
                Type::Signer { signer_id } => {
                    ("SELECT EXISTS(SELECT 1 FROM signers WHERE signer_id = ? LIMIT 1);", [signer_id.to_hex()])
                },
                Type::MySharedSigner { my_shared_signer_id } => {
                    ("SELECT EXISTS(SELECT 1 FROM my_shared_signers WHERE shared_signer_id = ? LIMIT 1);", [my_shared_signer_id.to_hex()])
                },
                Type::SharedSigner { shared_signer_id } => {
                    ("SELECT EXISTS(SELECT 1 FROM shared_signers WHERE shared_signer_id = ? LIMIT 1);", [shared_signer_id.to_hex()])
                }
            };

            let mut stmt = conn.prepare_cached(
                sql,
            )?;
            let mut rows = stmt.query(params)?;
            let exists: u8 = match rows.next()? {
                Some(row) => row.get(0)?,
                None => 0,
            };
            Ok(exists == 1)
        })
        .await?
    }

    pub async fn save_completed_proposal(
        &self,
        completed_proposal_id: EventId,
        policy_id: EventId,
        completed_proposal: CompletedProposal,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            conn.execute(
                        "INSERT OR IGNORE INTO completed_proposals (completed_proposal_id, policy_id, completed_proposal) VALUES (?, ?, ?);",
                        (completed_proposal_id.to_hex(), policy_id.to_hex(), completed_proposal.encrypt(&cipher)?),
                    )?;
                    tracing::info!("Completed proposal {completed_proposal_id} saved");
                    Ok(())
        }).await?
    }

    pub async fn completed_proposals(&self) -> Result<Vec<GetCompletedProposal>, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
            "SELECT completed_proposal_id, policy_id, completed_proposal FROM completed_proposals;",
        )?;
            let mut rows = stmt.query([])?;
            let mut proposals = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let proposal_id: String = row.get(0)?;
                let policy_id: String = row.get(1)?;
                let proposal: Vec<u8> = row.get(2)?;
                proposals.push(GetCompletedProposal {
                    policy_id: EventId::from_hex(policy_id)?,
                    completed_proposal_id: EventId::from_hex(proposal_id)?,
                    proposal: CompletedProposal::decrypt(&cipher, proposal)?,
                });
            }
            Ok(proposals)
        })
        .await?
    }

    pub async fn completed_proposals_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<GetCompletedProposal>, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
        let mut stmt = conn.prepare_cached(
                    "SELECT completed_proposal_id, completed_proposal FROM completed_proposals WHERE policy_id = ?;",
                )?;
                let mut rows = stmt.query([policy_id.to_hex()])?;
                let mut proposals = Vec::new();
                while let Ok(Some(row)) = rows.next() {
                    let proposal_id: String = row.get(0)?;
                    let proposal: Vec<u8> = row.get(1)?;
                    proposals.push(GetCompletedProposal {
                        policy_id,
                        completed_proposal_id: EventId::from_hex(proposal_id)?,
                        proposal: CompletedProposal::decrypt(&cipher, proposal)?,
                    });
                }
                Ok(proposals)
        }).await?
    }

    pub async fn get_completed_proposal(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<GetCompletedProposal, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT policy_id, completed_proposal FROM completed_proposals WHERE completed_proposal_id = ? LIMIT 1;")?;
            let mut rows = stmt.query([completed_proposal_id.to_hex()])?;
            let row = rows
                .next()?
                .ok_or(Error::NotFound("completed proposal".into()))?;
            let policy_id: String = row.get(0)?;
            let proposal: Vec<u8> = row.get(1)?;
            Ok(GetCompletedProposal {
                policy_id: EventId::from_hex(policy_id)?,
                completed_proposal_id,
                proposal: CompletedProposal::decrypt(&cipher, proposal)?,
            })
        }).await?
    }

    pub async fn delete_completed_proposal(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "DELETE FROM completed_proposals WHERE completed_proposal_id = ?;",
                [completed_proposal_id.to_hex()],
            )?;
            tracing::info!("Deleted completed proposal {completed_proposal_id}");
            Ok(())
        })
        .await?
    }

    pub async fn get_description_by_txid(
        &self,
        policy_id: EventId,
        txid: Txid,
    ) -> Result<Option<String>, Error> {
        for GetCompletedProposal { proposal, .. } in self
            .completed_proposals_by_policy_id(policy_id)
            .await?
            .into_iter()
        {
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

    pub async fn get_txs_descriptions(
        &self,
        policy_id: EventId,
    ) -> Result<HashMap<Txid, String>, Error> {
        let mut map = HashMap::new();
        for GetCompletedProposal { proposal, .. } in self
            .completed_proposals_by_policy_id(policy_id)
            .await?
            .into_iter()
        {
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

    pub async fn delete_generic_event_id(&self, event_id: EventId) -> Result<(), Error> {
        if self
            .exists(Type::CompletedProposal {
                completed_proposal_id: event_id,
            })
            .await?
        {
            self.delete_completed_proposal(event_id).await?;
        } else if self
            .exists(Type::Signer {
                signer_id: event_id,
            })
            .await?
        {
            self.delete_signer(event_id).await?;
        } else if self
            .exists(Type::MySharedSigner {
                my_shared_signer_id: event_id,
            })
            .await?
            || self
                .exists(Type::SharedSigner {
                    shared_signer_id: event_id,
                })
                .await?
        {
            self.delete_shared_signer(event_id).await?;
        };

        Ok(())
    }

    pub async fn save_pending_event(&self, event: Event) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("INSERT OR IGNORE INTO pending_events (event) VALUES (?);")?;
            stmt.execute([event.as_json()])?;
            tracing::info!("Saved pending event {} (kind={:?})", event.id, event.kind);
            Ok(())
        })
        .await?
    }

    pub async fn get_pending_events(&self) -> Result<Vec<Event>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT event FROM pending_events;")?;
            let mut rows = stmt.query([])?;
            let mut events: Vec<Event> = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let json: String = row.get(0)?;
                let event: Event = Event::from_json(json)?;
                events.push(event);
            }
            Ok(events)
        })
        .await?
    }
}
