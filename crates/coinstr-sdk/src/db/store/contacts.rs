// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{Metadata, Timestamp};

use super::{PooledConnection, Store};
use crate::constants::METADATA_SYNC_INTERVAL;
use crate::db::Error;
use crate::util;

impl Store {
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
}
