// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::net::SocketAddr;

use nostr_sdk::{Timestamp, Url};

use super::Store;
use crate::db::Error;

impl Store {
    pub fn save_last_relay_sync(&self, relay_url: &Url, timestamp: Timestamp) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let last_sync: u64 = timestamp.as_u64();
        let mut stmt = conn.prepare_cached("INSERT INTO relays (url, enabled, last_sync) VALUES (?, ?, ?) ON CONFLICT(url) DO UPDATE SET last_sync = ?;")?;
        stmt.execute((relay_url, false, last_sync, last_sync))?;
        Ok(())
    }

    pub fn get_last_relay_sync(&self, relay_url: &Url) -> Result<Timestamp, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT last_sync FROM relays WHERE url = ?")?;
        let mut rows = stmt.query([relay_url])?;
        let row = rows.next()?.ok_or(Error::NotFound("relay".into()))?;
        let last_sync: Option<u64> = row.get(0)?;
        let last_sync: u64 = last_sync.unwrap_or_default();
        Ok(Timestamp::from(last_sync))
    }

    pub fn insert_relay(&self, url: &Url, proxy: Option<SocketAddr>) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute(
            "INSERT OR IGNORE INTO relays (url, proxy) VALUES (?, ?);",
            (url, proxy.map(|a| a.to_string())),
        )?;
        Ok(())
    }

    pub fn get_relays(&self, enabled: bool) -> Result<Vec<(Url, Option<SocketAddr>)>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare("SELECT url, proxy FROM relays WHERE enabled = ?")?;
        let mut rows = stmt.query([enabled])?;

        let mut relays: Vec<(Url, Option<SocketAddr>)> = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let url: Url = row.get(0)?;
            let proxy: Option<String> = row.get(1)?;
            relays.push((
                url,
                proxy
                    .map(|p| p.parse())
                    .filter(|r| r.is_ok())
                    .map(|r| r.unwrap()),
            ));
        }
        Ok(relays)
    }

    pub fn delete_relay(&self, url: &Url) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute("DELETE FROM relays WHERE url = ?;", [url])?;
        Ok(())
    }

    pub fn enable_relay(&self, url: &Url) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute("UPDATE relays SET enabled = ? WHERE url = ?;", (1, url))?;
        Ok(())
    }

    pub fn disable_relay(&self, url: &Url) -> Result<(), Error> {
        let conn = self.pool.get()?;
        conn.execute("UPDATE relays SET enabled = ? WHERE url = ?;", (0, url))?;
        Ok(())
    }
}