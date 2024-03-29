// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::str::FromStr;

use smartvaults_protocol::nostr::nips::nip46::{Message as NIP46Message, NostrConnectURI};
use smartvaults_protocol::nostr::{EventId, JsonUtil, PublicKey, Timestamp, Url};

use super::Store;
use crate::model::NostrConnectRequest;
use crate::Error;

impl Store {
    pub async fn save_nostr_connect_uri(&self, uri: NostrConnectURI) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "INSERT OR IGNORE INTO nostr_connect_sessions (app_public_key, uri, timestamp) VALUES (?, ?, ?);",
                (uri.public_key.to_string(), uri.to_string(), Timestamp::now().as_u64()),
            )?;
            Ok(())
        }).await?
    }

    pub async fn nostr_connect_session_exists(
        &self,
        app_public_key: PublicKey,
    ) -> Result<bool, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
            "SELECT EXISTS(SELECT 1 FROM nostr_connect_sessions WHERE app_public_key = ? LIMIT 1);",
        )?;
            let mut rows = stmt.query([app_public_key.to_string()])?;
            let exists: u8 = match rows.next()? {
                Some(row) => row.get(0)?,
                None => 0,
            };
            Ok(exists == 1)
        })
        .await?
    }

    pub async fn save_nostr_connect_request(
        &self,
        event_id: EventId,
        app_public_key: PublicKey,
        message: NIP46Message,
        timestamp: Timestamp,
        approved: bool,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
            "INSERT OR IGNORE INTO nostr_connect_requests (event_id, app_public_key, message, timestamp, approved) VALUES (?, ?, ?, ?, ?);",
            (event_id.to_hex(), app_public_key.to_string(), message.as_json(), timestamp.as_u64(), approved),
        )?;
        Ok(())
        }).await?
    }

    pub async fn get_nostr_connect_sessions(
        &self,
    ) -> Result<Vec<(NostrConnectURI, Timestamp)>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT uri, timestamp FROM nostr_connect_sessions;")?;
            let mut rows = stmt.query([])?;
            let mut sessions: Vec<(NostrConnectURI, Timestamp)> = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let uri: String = row.get(0)?;
                let uri: NostrConnectURI = NostrConnectURI::from_str(&uri)?;
                let timestamp: u64 = row.get(1)?;
                sessions.push((uri, Timestamp::from(timestamp)));
            }
            Ok(sessions)
        })
        .await?
    }

    pub async fn get_nostr_connect_sessions_relays(&self) -> Result<Vec<Url>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT uri FROM nostr_connect_sessions;")?;
            let mut rows = stmt.query([])?;
            let mut urls = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let uri: String = row.get(0)?;
                let uri: NostrConnectURI = NostrConnectURI::from_str(&uri)?;
                if !urls.contains(&uri.relay_url) {
                    urls.push(uri.relay_url);
                }
            }
            Ok(urls)
        })
        .await?
    }

    pub async fn get_nostr_connect_session(
        &self,
        app_public_key: PublicKey,
    ) -> Result<NostrConnectURI, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT uri FROM nostr_connect_sessions WHERE app_public_key = ?;",
            )?;
            let mut rows = stmt.query([app_public_key.to_string()])?;
            let row = rows
                .next()?
                .ok_or(Error::NotFound("nostr connect session".into()))?;
            let uri: String = row.get(0)?;
            Ok(NostrConnectURI::from_str(&uri)?)
        })
        .await?
    }

    pub async fn delete_nostr_connect_session(
        &self,
        app_public_key: PublicKey,
    ) -> Result<(), Error> {
        // Delete notifications
        // self.delete_notification(policy_id)?;

        // Delete session
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "DELETE FROM nostr_connect_sessions WHERE app_public_key = ?;",
                [app_public_key.to_string()],
            )?;
            conn.execute(
                "DELETE FROM nostr_connect_requests WHERE app_public_key = ?;",
                [app_public_key.to_string()],
            )?;
            tracing::info!("Deleted nostr connect session {app_public_key}");
            Ok(())
        })
        .await?
    }

    pub async fn get_nostr_connect_requests(
        &self,
        approved: bool,
    ) -> Result<Vec<NostrConnectRequest>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT event_id, app_public_key, message, timestamp, approved FROM nostr_connect_requests WHERE approved = ? ORDER BY timestamp DESC;")?;
        let mut rows = stmt.query([approved])?;
        let mut requests = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let event_id: String = row.get(0)?;
            let app_public_key: String = row.get(1)?;
            let message: String = row.get(2)?;
            let timestamp: u64 = row.get(3)?;
            let approved: bool = row.get(4)?;
            requests.push(NostrConnectRequest {
                event_id: EventId::from_hex(event_id)?,
                app_public_key: PublicKey::from_str(&app_public_key)?,
                message: NIP46Message::from_json(message)?,
                timestamp: Timestamp::from(timestamp),
                approved,
            });
        }
        Ok(requests)
        }).await?
    }

    pub async fn get_nostr_connect_request(
        &self,
        event_id: EventId,
    ) -> Result<NostrConnectRequest, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT app_public_key, message, timestamp, approved FROM nostr_connect_requests WHERE event_id = ?;")?;
        let mut rows = stmt.query([event_id.to_hex()])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("nostr connect request".into()))?;
        let app_public_key: String = row.get(0)?;
        let message: String = row.get(1)?;
        let timestamp: u64 = row.get(2)?;
        let approved: bool = row.get(3)?;
        Ok(NostrConnectRequest {
            event_id,
            app_public_key: PublicKey::from_str(&app_public_key)?,
            message: NIP46Message::from_json(message)?,
            timestamp: Timestamp::from(timestamp),
            approved,
        })
        }).await?
    }

    pub async fn set_nostr_connect_request_as_approved(
        &self,
        event_id: EventId,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "UPDATE nostr_connect_requests SET approved = 1 WHERE event_id = ?",
            )?;
            stmt.execute([event_id.to_hex()])?;
            Ok(())
        })
        .await?
    }

    pub async fn set_nostr_connect_auto_approve(
        &self,
        app_public_key: PublicKey,
        until: Timestamp,
    ) {
        let mut nostr_connect_auto_approve = self.nostr_connect_auto_approve.write().await;
        nostr_connect_auto_approve.insert(app_public_key, until);
    }

    pub async fn is_nostr_connect_session_pre_authorized(&self, app_public_key: PublicKey) -> bool {
        let mut nostr_connect_auto_approve = self.nostr_connect_auto_approve.write().await;
        if let Some(until) = nostr_connect_auto_approve.get(&app_public_key) {
            if Timestamp::now() < *until {
                return true;
            } else {
                nostr_connect_auto_approve.remove(&app_public_key);
            }
        }
        false
    }

    pub async fn revoke_nostr_connect_auto_approve(&self, app_public_key: PublicKey) {
        let mut nostr_connect_auto_approve = self.nostr_connect_auto_approve.write().await;
        nostr_connect_auto_approve.remove(&app_public_key);
    }

    pub async fn get_nostr_connect_pre_authorizations(&self) -> BTreeMap<PublicKey, Timestamp> {
        let nostr_connect_auto_approve = self.nostr_connect_auto_approve.read().await;
        nostr_connect_auto_approve
            .iter()
            .map(|(pk, ts)| (*pk, *ts))
            .collect()
    }

    pub async fn delete_nostr_connect_request(&self, event_id: EventId) -> Result<(), Error> {
        // Delete notifications
        // self.delete_notification(policy_id)?;

        // Delete
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "DELETE FROM nostr_connect_requests WHERE event_id = ?;",
                [event_id.to_hex()],
            )?;
            tracing::info!("Deleted nostr connect request {event_id}");
            Ok(())
        })
        .await?
    }
}
