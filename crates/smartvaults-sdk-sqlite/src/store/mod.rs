// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Store

#![allow(clippy::type_complexity)]

use std::collections::HashMap;
use std::fmt::Debug;
use std::path::Path;
use std::sync::Arc;

use chacha20poly1305::aead::KeyInit;
use chacha20poly1305::XChaCha20Poly1305;
use deadpool_sqlite::{Config, Object, Pool, Runtime};
use rusqlite::config::DbConfig;
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
use super::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Type {
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

    pub async fn delete_generic_event_id(&self, event_id: EventId) -> Result<(), Error> {
        if self
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
