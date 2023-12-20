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
use smartvaults_protocol::nostr::secp256k1::XOnlyPublicKey;
use smartvaults_protocol::nostr::{Keys, Timestamp};
use tokio::sync::RwLock;

mod connect;
mod relays;
mod timechain;

use super::encryption::StoreEncryption;
use super::migration::{self, STARTUP_SQL};
use super::Error;

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
}
