// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::net::SocketAddr;

use smartvaults_protocol::nostr::{Timestamp, Url};

use crate::{Error, Store};

impl Store {
    pub async fn save_last_relay_sync(
        &self,
        relay_url: Url,
        timestamp: Timestamp,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let last_sync: u64 = timestamp.as_u64();
            let mut stmt = conn.prepare_cached("INSERT INTO relays (url, enabled, last_sync) VALUES (?, ?, ?) ON CONFLICT(url) DO UPDATE SET last_sync = ?;")?;
            stmt.execute((relay_url.as_str(), false, last_sync, last_sync))?;
            Ok(())
        }).await?
    }

    pub async fn get_last_relay_sync(&self, relay_url: Url) -> Result<Timestamp, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT last_sync FROM relays WHERE url = ?")?;
            let mut rows = stmt.query([relay_url.as_str()])?;
            let row = rows.next()?.ok_or(Error::NotFound("relay".into()))?;
            let last_sync: Option<u64> = row.get(0)?;
            let last_sync: u64 = last_sync.unwrap_or_default();
            Ok(Timestamp::from(last_sync))
        })
        .await?
    }

    pub async fn insert_relay(&self, url: Url, proxy: Option<SocketAddr>) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO relays (url, proxy) VALUES (?, ?);",
                (url.as_str(), proxy.map(|a| a.to_string())),
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn get_relays(&self, enabled: bool) -> Result<Vec<(Url, Option<SocketAddr>)>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT url, proxy FROM relays WHERE enabled = ?")?;
            let mut rows = stmt.query([enabled])?;

            let mut relays: Vec<(Url, Option<SocketAddr>)> = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let url: String = row.get(0)?;
                let proxy: Option<String> = row.get(1)?;
                relays.push((
                    Url::parse(&url)?,
                    proxy
                        .map(|p| p.parse())
                        .filter(|r| r.is_ok())
                        .map(|r| r.unwrap()),
                ));
            }
            Ok(relays)
        })
        .await?
    }

    pub async fn delete_relay(&self, url: Url) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute("DELETE FROM relays WHERE url = ?;", [url.as_str()])?;
            Ok(())
        })
        .await?
    }

    pub async fn enable_relay(&self, url: Url) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "UPDATE relays SET enabled = ? WHERE url = ?;",
                (1, url.as_str()),
            )?;
            Ok(())
        })
        .await?
    }

    pub async fn disable_relay(&self, url: Url) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "UPDATE relays SET enabled = ? WHERE url = ?;",
                (0, url.as_str()),
            )?;
            Ok(())
        })
        .await?
    }
}
