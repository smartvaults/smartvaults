// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};
use std::str::FromStr;

use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::SharedSigner;
use smartvaults_protocol::nostr::EventId;

use super::{Error, Store, StoreEncryption};
use crate::model::GetSharedSignerRaw;

impl Store {
    pub async fn delete_shared_signer(&self, shared_signer_id: EventId) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            conn.execute(
                "DELETE FROM my_shared_signers WHERE shared_signer_id = ?;",
                [shared_signer_id.to_hex()],
            )?;
            conn.execute(
                "DELETE FROM shared_signers WHERE shared_signer_id = ?;",
                [shared_signer_id.to_hex()],
            )?;
            tracing::info!("Deleted shared signer {shared_signer_id}");
            Ok(())
        })
        .await?
    }

    pub async fn my_shared_signer_already_shared(
        &self,
        signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<bool, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT EXISTS(SELECT 1 FROM my_shared_signers WHERE signer_id = ? AND public_key = ? LIMIT 1);",
            )?;
            let mut rows = stmt.query([signer_id.to_hex(), public_key.to_string()])?;
            let exists: u8 = match rows.next()? {
                Some(row) => row.get(0)?,
                None => 0,
            };
            Ok(exists == 1)
        }).await?
    }

    pub async fn save_my_shared_signer(
        &self,
        signer_id: EventId,
        shared_signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn
                .prepare_cached("INSERT OR IGNORE INTO my_shared_signers (signer_id, shared_signer_id, public_key) VALUES (?, ?, ?);")?;
            stmt.execute((
                signer_id.to_hex(),
                shared_signer_id.to_hex(),
                public_key.to_string(),
            ))?;
            tracing::info!("Saved my shared signer {shared_signer_id} (signer {signer_id})");
            Ok(())
        }).await?
    }

    pub async fn save_shared_signer(
        &self,
        shared_signer_id: EventId,
        owner_public_key: XOnlyPublicKey,
        shared_signer: SharedSigner,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt = conn
                .prepare_cached("INSERT OR IGNORE INTO shared_signers (shared_signer_id, owner_public_key, shared_signer) VALUES (?, ?, ?);")?;
            stmt.execute((
                shared_signer_id.to_hex(),
                owner_public_key.to_string(),
                shared_signer.encrypt(&cipher)?,
            ))?;
            tracing::info!("Saved shared signer {shared_signer_id}");
            Ok(())
        }).await?
    }

    pub async fn get_public_key_for_my_shared_signer(
        &self,
        shared_signer_id: EventId,
    ) -> Result<XOnlyPublicKey, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT public_key FROM my_shared_signers WHERE shared_signer_id = ? LIMIT 1;",
            )?;
            let mut rows = stmt.query([shared_signer_id.to_hex()])?;
            let row = rows
                .next()?
                .ok_or(Error::NotFound("my shared signer".into()))?;
            let public_key: String = row.get(0)?;
            Ok(XOnlyPublicKey::from_str(&public_key)?)
        })
        .await?
    }

    pub async fn get_my_shared_signers(&self) -> Result<BTreeMap<EventId, XOnlyPublicKey>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT shared_signer_id, public_key FROM my_shared_signers;")?;
            let mut rows = stmt.query([])?;
            let mut map = BTreeMap::new();
            while let Ok(Some(row)) = rows.next() {
                let shared_signer_id: String = row.get(0)?;
                let public_key: String = row.get(1)?;
                map.insert(
                    EventId::from_hex(shared_signer_id)?,
                    XOnlyPublicKey::from_str(&public_key)?,
                );
            }
            Ok(map)
        })
        .await?
    }

    pub async fn get_my_shared_signers_by_signer_id(
        &self,
        signer_id: EventId,
    ) -> Result<BTreeMap<EventId, XOnlyPublicKey>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT shared_signer_id, public_key FROM my_shared_signers WHERE signer_id = ?;",
            )?;
            let mut rows = stmt.query([signer_id.to_hex()])?;
            let mut map = BTreeMap::new();
            while let Ok(Some(row)) = rows.next() {
                let shared_signer_id: String = row.get(0)?;
                let public_key: String = row.get(1)?;
                map.insert(
                    EventId::from_hex(shared_signer_id)?,
                    XOnlyPublicKey::from_str(&public_key)?,
                );
            }
            Ok(map)
        })
        .await?
    }

    pub async fn get_owner_public_key_for_shared_signer(
        &self,
        shared_signer_id: EventId,
    ) -> Result<XOnlyPublicKey, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT owner_public_key FROM shared_signers WHERE shared_signer_id = ? LIMIT 1;",
            )?;
            let mut rows = stmt.query([shared_signer_id.to_hex()])?;
            let row = rows
                .next()?
                .ok_or(Error::NotFound("shared signer".into()))?;
            let public_key: String = row.get(0)?;
            Ok(XOnlyPublicKey::from_str(&public_key)?)
        })
        .await?
    }

    pub async fn get_shared_signers(&self) -> Result<Vec<GetSharedSignerRaw>, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT shared_signer_id, owner_public_key, shared_signer FROM shared_signers;",
            )?;
            let mut rows = stmt.query([])?;
            let mut list = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let shared_signer_id: String = row.get(0)?;
                let public_key: String = row.get(1)?;
                let shared_signer: Vec<u8> = row.get(2)?;
                list.push(GetSharedSignerRaw {
                    shared_signer_id: EventId::from_hex(shared_signer_id)?,
                    owner_public_key: XOnlyPublicKey::from_str(&public_key)?,
                    shared_signer: SharedSigner::decrypt(&cipher, shared_signer)?,
                });
            }
            Ok(list)
        })
        .await?
    }

    pub async fn get_shared_signers_public_keys(&self) -> Result<HashSet<XOnlyPublicKey>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached("SELECT owner_public_key FROM shared_signers;")?;
            let mut rows = stmt.query([])?;
            let mut list = HashSet::new();
            while let Ok(Some(row)) = rows.next() {
                let public_key: String = row.get(0)?;
                list.insert(XOnlyPublicKey::from_str(&public_key)?);
            }
            Ok(list)
        })
        .await?
    }

    pub async fn get_shared_signers_by_public_key(
        &self,
        public_key: XOnlyPublicKey,
    ) -> Result<Vec<GetSharedSignerRaw>, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt = conn.prepare_cached(
                "SELECT shared_signer_id, shared_signer FROM shared_signers WHERE owner_public_key = ?;",
            )?;
            let mut rows = stmt.query([public_key.to_string()])?;
            let mut list = Vec::new();
            while let Ok(Some(row)) = rows.next() {
                let shared_signer_id: String = row.get(0)?;
                let shared_signer: Vec<u8> = row.get(1)?;
                list.push(GetSharedSignerRaw {
                    shared_signer_id: EventId::from_hex(shared_signer_id)?,
                    owner_public_key: public_key,
                    shared_signer: SharedSigner::decrypt(&cipher, shared_signer)?,
                });
            }
            Ok(list)
        }).await?
    }
}
