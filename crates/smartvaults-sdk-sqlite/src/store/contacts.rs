// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::time::Duration;

use rusqlite::Connection;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_protocol::nostr::{JsonUtil, Metadata, Timestamp};

use crate::{Error, Store};

impl Store {
    pub async fn get_contacts_public_keys(&self) -> Result<HashSet<XOnlyPublicKey>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT public_key FROM contacts;")?;
            let mut rows = stmt.query([])?;
            let mut public_keys = HashSet::new();
            while let Ok(Some(row)) = rows.next() {
                let public_key: String = row.get(0)?;
                public_keys.insert(XOnlyPublicKey::from_str(&public_key)?);
            }
            Ok(public_keys)
        })
        .await?
    }

    pub async fn delete_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "DELETE FROM contacts WHERE public_key = ?;",
                [public_key.to_string()],
            )?;
            tracing::info!("Deleted contact {public_key}");
            Ok(())
        })
        .await?
    }

    pub async fn save_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        if public_key != self.public_key {
            let conn = self.acquire().await?;
            conn.interact(move |conn| {
                let mut stmt =
                    conn.prepare_cached("INSERT OR IGNORE INTO contacts (public_key) VALUES (?);")?;
                stmt.execute([public_key.to_string()])?;
                let mut stmt = conn.prepare_cached(
                    "INSERT OR IGNORE INTO metadata (public_key, metadata) VALUES (?, ?);",
                )?;
                stmt.execute([public_key.to_string(), Metadata::default().as_json()])?;
                tracing::info!("Saved contact {public_key}");
                Ok::<(), Error>(())
            })
            .await??;
        }

        Ok(())
    }

    pub async fn save_contacts(&self, contacts: HashSet<XOnlyPublicKey>) -> Result<(), Error> {
        let public_keys = self.get_contacts_public_keys().await?;

        for pk in public_keys.difference(&contacts) {
            self.delete_contact(*pk).await?;
        }

        for public_key in contacts.difference(&public_keys) {
            self.save_contact(*public_key).await?;
        }

        Ok(())
    }

    fn get_metadata_with_conn(
        &self,
        conn: &Connection,
        public_key: XOnlyPublicKey,
    ) -> Result<Metadata, Error> {
        let mut stmt =
            conn.prepare_cached("SELECT metadata FROM metadata WHERE public_key = ?;")?;
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

    pub async fn get_metadata(&self, public_key: XOnlyPublicKey) -> Result<Metadata, Error> {
        let conn = self.acquire().await?;
        let this = self.clone();
        conn.interact(move |conn| this.get_metadata_with_conn(conn, public_key))
            .await?
    }

    pub async fn get_contacts_with_metadata(
        &self,
    ) -> Result<Vec<(XOnlyPublicKey, Metadata)>, Error> {
        let conn = self.acquire().await?;
        let this = self.clone();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT public_key FROM contacts;")?;
            let mut rows = stmt.query([])?;
            let mut contacts = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let public_key: String = row.get(0)?;
                let public_key = XOnlyPublicKey::from_str(&public_key)?;
                contacts.push((public_key, this.get_metadata_with_conn(conn, public_key)?));
            }
            Ok(contacts)
        })
        .await?
    }

    pub async fn get_known_public_keys_with_metadata(
        &self,
    ) -> Result<HashMap<XOnlyPublicKey, Metadata>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT public_key, metadata FROM metadata;")?;
            let mut rows = stmt.query([])?;
            let mut public_keys = HashMap::new();
            while let Ok(Some(row)) = rows.next() {
                let public_key: String = row.get(0)?;
                let metadata: Option<String> = row.get(1)?;
                if let Some(metadata) = metadata {
                    let public_key = XOnlyPublicKey::from_str(&public_key)?;
                    public_keys.insert(public_key, Metadata::from_json(metadata)?);
                }
            }
            Ok(public_keys)
        })
        .await?
    }

    pub async fn get_unsynced_metadata_pubkeys(
        &self,
        sync_interval: Duration,
    ) -> Result<Vec<XOnlyPublicKey>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT public_key, last_sync FROM metadata;")?;
            let mut rows = stmt.query([])?;
            let mut public_keys: Vec<XOnlyPublicKey> = Vec::new();
            let now = Timestamp::now();
            while let Ok(Some(row)) = rows.next() {
                let public_key: String = row.get(0)?;
                let public_key: XOnlyPublicKey = XOnlyPublicKey::from_str(&public_key)?;
                let last_sync: Option<u64> = row.get(1)?;

                if let Some(last_sync) = last_sync {
                    if last_sync + sync_interval.as_secs() < now.as_u64() {
                        public_keys.push(public_key);
                    }
                } else {
                    public_keys.push(public_key);
                }
            }
            Ok(public_keys)
        })
        .await?
    }

    pub async fn set_metadata(
        &self,
        public_key: XOnlyPublicKey,
        metadata: Metadata,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
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
        }).await?
    }
}
