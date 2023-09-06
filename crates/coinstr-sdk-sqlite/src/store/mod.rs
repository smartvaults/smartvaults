// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//! Store

#![allow(clippy::type_complexity)]

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;

use chacha20poly1305::aead::KeyInit;
use chacha20poly1305::XChaCha20Poly1305;
use coinstr_core::bitcoin::{Network, Txid};
use coinstr_core::proposal::{CompletedProposal, Proposal};
use coinstr_core::ApprovedProposal;
use coinstr_protocol::nostr::event::id::EventId;
use coinstr_protocol::nostr::secp256k1::{SecretKey, XOnlyPublicKey};
use coinstr_protocol::nostr::{Event, Keys, Timestamp};
use deadpool_sqlite::{Config, Object, Pool, Runtime};
use rusqlite::config::DbConfig;
use rusqlite::Connection;
use tokio::sync::RwLock;

mod connect;
mod contacts;
mod label;
mod policy;
mod relays;
mod signers;
mod timechain;
mod utxos;

use super::encryption::StoreEncryption;
use super::migration::{self, STARTUP_SQL};
use super::model::{GetApproval, GetApprovedProposals, GetCompletedProposal, GetProposal};
use super::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
    SharedKey { policy_id: EventId },
    Policy { policy_id: EventId },
    Proposal { proposal_id: EventId },
    ApprovedProposal { approval_id: EventId },
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
    public_key: XOnlyPublicKey,
    network: Network,
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
    pub async fn open<P>(user_db_path: P, keys: &Keys, network: Network) -> Result<Self, Error>
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
            public_key: keys.public_key(),
            network,
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
                Type::SharedKey { policy_id } => {
                    ("SELECT EXISTS(SELECT 1 FROM shared_keys WHERE policy_id = ? LIMIT 1);", [policy_id.to_hex()])
                }
                Type::Policy { policy_id } => {
                    ("SELECT EXISTS(SELECT 1 FROM policies WHERE policy_id = ? LIMIT 1);", [policy_id.to_hex()])
                }
                Type::Proposal { proposal_id } => {
                    ("SELECT EXISTS(SELECT 1 FROM proposals WHERE proposal_id = ? LIMIT 1);", [proposal_id.to_hex()])
                }
                Type::ApprovedProposal { approval_id } => {
                    ("SELECT EXISTS(SELECT 1 FROM approved_proposals WHERE approval_id = ? LIMIT 1);", [approval_id.to_hex()])
                },
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

    pub async fn save_shared_key(&self, policy_id: EventId, shared_key: Keys) -> Result<(), Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO shared_keys (policy_id, shared_key) VALUES (?, ?);",
                (
                    policy_id.to_hex(),
                    shared_key.secret_key()?.encrypt(&cipher)?,
                ),
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn get_shared_key(&self, policy_id: EventId) -> Result<Keys, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT shared_key FROM shared_keys WHERE policy_id = ?;")?;
            let mut rows = stmt.query([policy_id.to_hex()])?;
            let row = rows.next()?.ok_or(Error::NotFound("shared_key".into()))?;
            let sk: Vec<u8> = row.get(0)?;
            let sk = SecretKey::decrypt(&cipher, sk)?;
            Ok(Keys::new(sk))
        })
        .await?
    }

    pub async fn get_nostr_pubkeys(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<XOnlyPublicKey>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn
                .prepare_cached("SELECT public_key FROM nostr_public_keys WHERE policy_id = ?;")?;
            let mut rows = stmt.query([policy_id.to_hex()])?;
            let mut pubkeys = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let public_key: String = row.get(0)?;
                pubkeys.push(XOnlyPublicKey::from_str(&public_key)?);
            }
            Ok(pubkeys)
        })
        .await?
    }

    pub async fn save_proposal(
        &self,
        proposal_id: EventId,
        policy_id: EventId,
        proposal: Proposal,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        let psbt = proposal.psbt();
        conn.interact(move |conn| {
            conn.execute(
            "INSERT OR IGNORE INTO proposals (proposal_id, policy_id, proposal) VALUES (?, ?, ?);",
            (
                proposal_id.to_hex(),
                policy_id.to_hex(),
                proposal.encrypt(&cipher)?,
            ),
        )?;
            Ok::<(), Error>(())
        })
        .await??;

        // Freeze UTXOs
        for txin in psbt.unsigned_tx.input.into_iter() {
            self.freeze_utxo(txin.previous_output, policy_id, Some(proposal_id))
                .await?;
        }

        tracing::info!("Spending proposal {proposal_id} saved");
        Ok(())
    }

    pub async fn get_proposals(&self) -> Result<Vec<GetProposal>, Error> {
        let conn = self.acquire().await?;
        let this = self.clone();
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT proposal_id, policy_id, proposal FROM proposals;")?;
            let mut rows = stmt.query([])?;
            let mut proposals = Vec::new();

            while let Ok(Some(row)) = rows.next() {
                let proposal_id: String = row.get(0)?;
                let policy_id: String = row.get(1)?;
                let proposal: Vec<u8> = row.get(2)?;

                let proposal_id = EventId::from_hex(proposal_id)?;
                let policy_id = EventId::from_hex(policy_id)?;
                let proposal = Proposal::decrypt(&this.cipher, proposal)?;
                let approved_proposals =
                    this.get_approved_proposals_by_proposal_id(proposal_id, conn)?;

                proposals.push(GetProposal {
                    proposal_id,
                    policy_id,
                    signed: proposal.finalize(approved_proposals, this.network).is_ok(),
                    proposal,
                });
            }

            proposals.sort();

            Ok(proposals)
        })
        .await?
    }

    async fn get_proposal_ids_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<EventId>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT proposal_id FROM proposals WHERE policy_id = ?;")?;
            let mut rows = stmt.query([policy_id.to_hex()])?;
            let mut ids = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let proposal_id: String = row.get(0)?;
                ids.push(EventId::from_hex(proposal_id)?);
            }
            Ok(ids)
        })
        .await?
    }

    pub async fn get_proposals_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<GetProposal>, Error> {
        let conn = self.acquire().await?;
        let this = self.clone();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT proposal_id, proposal FROM proposals WHERE policy_id = ?;",
            )?;
            let mut rows = stmt.query([policy_id.to_hex()])?;
            let mut proposals = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let proposal_id: String = row.get(0)?;
                let proposal: Vec<u8> = row.get(1)?;

                let proposal_id = EventId::from_hex(proposal_id)?;
                let proposal = Proposal::decrypt(&this.cipher, proposal)?;
                let approved_proposals =
                    this.get_approved_proposals_by_proposal_id(proposal_id, conn)?;

                proposals.push(GetProposal {
                    proposal_id,
                    policy_id,
                    signed: proposal.finalize(approved_proposals, this.network).is_ok(),
                    proposal,
                });
            }

            proposals.sort();

            Ok(proposals)
        })
        .await?
    }

    pub async fn get_proposal(&self, proposal_id: EventId) -> Result<GetProposal, Error> {
        let conn = self.acquire().await?;
        let this = self.clone();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT policy_id, proposal FROM proposals WHERE proposal_id = ? LIMIT 1;",
            )?;
            let mut rows = stmt.query([proposal_id.to_hex()])?;
            let row = rows.next()?.ok_or(Error::NotFound("proposal".into()))?;
            let policy_id: String = row.get(0)?;
            let proposal: Vec<u8> = row.get(1)?;

            let policy_id = EventId::from_hex(policy_id)?;
            let proposal = Proposal::decrypt(&this.cipher, proposal)?;
            let approved_proposals =
                this.get_approved_proposals_by_proposal_id(proposal_id, conn)?;

            Ok(GetProposal {
                proposal_id,
                policy_id,
                signed: proposal.finalize(approved_proposals, this.network).is_ok(),
                proposal,
            })
        })
        .await?
    }

    pub async fn delete_proposal(&self, proposal_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(proposal_id).await?;

        // Delete proposal
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "DELETE FROM proposals WHERE proposal_id = ?;",
                [proposal_id.to_hex()],
            )?;

            // Delete approvals
            conn.execute(
                "DELETE FROM approved_proposals WHERE proposal_id = ?;",
                [proposal_id.to_hex()],
            )?;

            // Delete frozen UTXOs
            conn.execute(
                "DELETE FROM frozen_utxos WHERE proposal_id = ?;",
                [proposal_id.to_hex()],
            )?;

            tracing::info!("Deleted proposal {proposal_id}");
            Ok(())
        })
        .await?
    }

    pub async fn get_approvals_by_proposal_id(
        &self,
        proposal_id: EventId,
    ) -> Result<Vec<GetApproval>, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT approval_id, public_key, approved_proposal, timestamp FROM approved_proposals WHERE proposal_id = ?;")?;
            let mut rows = stmt.query([proposal_id.to_hex()])?;
            let mut approvals = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let approval_id: String = row.get(0)?;
                let public_key: String = row.get(1)?;
                let approved_proposal: Vec<u8> = row.get(2)?;
                let timestamp: u64 = row.get(3)?;
                approvals.push(GetApproval {
                    approval_id: EventId::from_hex(approval_id)?,
                    public_key: XOnlyPublicKey::from_str(&public_key)?,
                    approved_proposal: ApprovedProposal::decrypt(&cipher, approved_proposal)?,
                    timestamp: Timestamp::from(timestamp),
                });
            }
            Ok(approvals)
        }).await?
    }

    #[tracing::instrument(skip_all, level = "trace")]
    fn get_approved_proposals_by_proposal_id(
        &self,
        proposal_id: EventId,
        conn: &Connection,
    ) -> Result<Vec<ApprovedProposal>, Error> {
        let mut stmt = conn.prepare_cached(
            "SELECT approved_proposal FROM approved_proposals WHERE proposal_id = ?;",
        )?;
        let mut rows = stmt.query([proposal_id.to_hex()])?;
        let mut approvals = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let approval: Vec<u8> = row.get(0)?;
            approvals.push(ApprovedProposal::decrypt(&self.cipher, approval)?);
        }
        Ok(approvals)
    }

    pub async fn get_approved_proposals_by_id(
        &self,
        proposal_id: EventId,
    ) -> Result<GetApprovedProposals, Error> {
        let GetProposal {
            policy_id,
            proposal,
            ..
        } = self.get_proposal(proposal_id).await?;
        let approved_proposals = self.get_approvals_by_proposal_id(proposal_id).await?;
        Ok(GetApprovedProposals {
            policy_id,
            proposal,
            approved_proposals: approved_proposals
                .iter()
                .map(
                    |GetApproval {
                         approved_proposal, ..
                     }| approved_proposal.clone(),
                )
                .collect(),
        })
    }

    pub async fn save_approved_proposal(
        &self,
        proposal_id: EventId,
        author: XOnlyPublicKey,
        approval_id: EventId,
        approved_proposal: ApprovedProposal,
        timestamp: Timestamp,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO approved_proposals (approval_id, proposal_id, public_key, approved_proposal, timestamp) VALUES (?, ?, ?, ?, ?);",
                (approval_id.to_hex(), proposal_id.to_hex(), author.to_string(), approved_proposal.encrypt(&cipher)?, timestamp.as_u64()),
            )?;
            Ok(())
        }).await?
    }

    pub async fn get_policy_id_by_approval_id(
        &self,
        approval_id: EventId,
    ) -> Result<EventId, Error> {
        let conn = self.acquire().await?;
        let proposal_id = conn
            .interact(move |conn| {
                let mut stmt = conn.prepare_cached(
                    "SELECT proposal_id FROM approved_proposals WHERE approval_id = ? LIMIT 1;",
                )?;
                let mut rows = stmt.query([approval_id.to_hex()])?;
                let row = rows.next()?.ok_or(Error::NotFound("approval".into()))?;
                let proposal_id: String = row.get(0)?;
                let proposal_id = EventId::from_hex(proposal_id)?;
                Ok::<EventId, Error>(proposal_id)
            })
            .await??;
        let GetProposal { policy_id, .. } = self.get_proposal(proposal_id).await?;
        Ok(policy_id)
    }

    pub async fn delete_approval(&self, approval_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(approval_id).await?;

        // Delete policy
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "DELETE FROM approved_proposals WHERE approval_id = ?;",
                [approval_id.to_hex()],
            )?;
            tracing::info!("Deleted approval {approval_id}");
            Ok(())
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

    async fn get_completed_proposal_ids_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<EventId>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
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
        })
        .await?
    }

    pub async fn delete_completed_proposal(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<(), Error> {
        self.set_event_as_deleted(completed_proposal_id).await?;

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
            .exists(Type::Policy {
                policy_id: event_id,
            })
            .await?
        {
            self.delete_policy(event_id).await?;
        } else if self
            .exists(Type::Proposal {
                proposal_id: event_id,
            })
            .await?
        {
            self.delete_proposal(event_id).await?;
        } else if self
            .exists(Type::ApprovedProposal {
                approval_id: event_id,
            })
            .await?
        {
            self.delete_approval(event_id).await?;
        } else if self
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
        } else {
            self.set_event_as_deleted(event_id).await?;
        };

        Ok(())
    }

    pub async fn save_event(&self, event: Event) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn
                .prepare_cached("INSERT OR IGNORE INTO events (event_id, event) VALUES (?, ?);")?;
            stmt.execute((event.id.to_hex(), event.as_json()))?;
            Ok(())
        })
        .await?
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_events(&self) -> Result<Vec<Event>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT event FROM events;")?;
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

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_event_by_id(&self, event_id: EventId) -> Result<Event, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT event FROM events WHERE event_id = ? LIMIT 1;")?;
            let mut rows = stmt.query([event_id.to_hex()])?;
            let row = rows.next()?.ok_or(Error::NotFound("event".into()))?;
            let json: String = row.get(0)?;
            Ok(Event::from_json(json)?)
        })
        .await?
    }

    pub async fn event_was_deleted(&self, event_id: EventId) -> Result<bool, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT EXISTS(SELECT 1 FROM events WHERE event_id = ? AND deleted = 1 LIMIT 1);",
            )?;
            let mut rows = stmt.query([event_id.to_hex()])?;
            let exists: u8 = match rows.next()? {
                Some(row) => row.get(0)?,
                None => 0,
            };
            Ok(exists == 1)
        })
        .await?
    }

    pub async fn set_event_as_deleted(&self, event_id: EventId) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("UPDATE events SET deleted = 1 WHERE event_id = ?")?;
            stmt.execute([event_id.to_hex()])?;
            Ok(())
        })
        .await?
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
