// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::str::FromStr;

use coinstr_core::bitcoin::XOnlyPublicKey;
use coinstr_core::miniscript::{Descriptor, DescriptorPublicKey};
use coinstr_core::{SharedSigner, Signer};
use nostr_sdk::EventId;

use super::{Error, Store};
use crate::db::model::{GetSharedSigner, GetSigner};
use crate::util::encryption::EncryptionWithKeys;

impl Store {
    pub fn signer_exists(&self, signer_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT EXISTS(SELECT 1 FROM signers WHERE signer_id = ? LIMIT 1);")?;
        let mut rows = stmt.query([signer_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_signer(&self, signer_id: EventId, signer: Signer) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("INSERT OR IGNORE INTO signers (signer_id, signer) VALUES (?, ?);")?;
        stmt.execute((signer_id.to_hex(), signer.encrypt_with_keys(&self.keys)?))?;
        tracing::info!("Saved signer {signer_id}");
        Ok(())
    }

    pub(crate) fn get_signers(&self) -> Result<Vec<GetSigner>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT signer_id, signer FROM signers;")?;
        let mut rows = stmt.query([])?;
        let mut list = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let signer_id: String = row.get(0)?;
            let signer: String = row.get(1)?;
            list.push(GetSigner {
                signer_id: EventId::from_hex(signer_id)?,
                signer: Signer::decrypt_with_keys(&self.keys, signer)?,
            });
        }
        Ok(list)
    }

    pub(crate) fn signer_descriptor_exists(
        &self,
        descriptor: Descriptor<DescriptorPublicKey>,
    ) -> Result<bool, Error> {
        let signers = self.get_signers()?;
        for GetSigner { signer, .. } in signers.into_iter() {
            if signer.descriptor() == descriptor {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub fn get_signer_by_id(&self, signer_id: EventId) -> Result<Signer, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT signer FROM signers WHERE signer_id = ?;")?;
        let mut rows = stmt.query([signer_id.to_hex()])?;
        let row = rows.next()?.ok_or(Error::NotFound("signer".into()))?;
        let signer: String = row.get(0)?;
        Ok(Signer::decrypt_with_keys(&self.keys, signer)?)
    }

    pub fn delete_signer(&self, signer_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(signer_id)?;

        // Delete notification
        //self.delete_notification(Notification::NewProposal(proposal_id))?;

        // Delete signer
        let conn = self.pool.get()?;
        conn.execute(
            "DELETE FROM signers WHERE signer_id = ?;",
            [signer_id.to_hex()],
        )?;

        conn.execute(
            "DELETE FROM my_shared_signers WHERE signer_id = ?;",
            [signer_id.to_hex()],
        )?;

        tracing::info!("Deleted signer {signer_id}");
        Ok(())
    }

    pub fn delete_shared_signer(&self, shared_signer_id: EventId) -> Result<(), Error> {
        self.set_event_as_deleted(shared_signer_id)?;

        // Delete notification
        self.delete_notification(shared_signer_id)?;

        let conn = self.pool.get()?;
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
    }

    pub fn my_shared_signer_exists(&self, shared_signer_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM my_shared_signers WHERE shared_signer_id = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([shared_signer_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn shared_signer_exists(&self, shared_signer_id: EventId) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM shared_signers WHERE shared_signer_id = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([shared_signer_id.to_hex()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn my_shared_signer_already_shared(
        &self,
        signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<bool, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT EXISTS(SELECT 1 FROM my_shared_signers WHERE signer_id = ? AND public_key = ? LIMIT 1);",
        )?;
        let mut rows = stmt.query([signer_id.to_hex(), public_key.to_string()])?;
        let exists: u8 = match rows.next()? {
            Some(row) => row.get(0)?,
            None => 0,
        };
        Ok(exists == 1)
    }

    pub fn save_my_shared_signer(
        &self,
        signer_id: EventId,
        shared_signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("INSERT OR IGNORE INTO my_shared_signers (signer_id, shared_signer_id, public_key) VALUES (?, ?, ?);")?;
        stmt.execute((
            signer_id.to_hex(),
            shared_signer_id.to_hex(),
            public_key.to_string(),
        ))?;
        tracing::info!("Saved my shared signer {shared_signer_id} (signer {signer_id})");
        Ok(())
    }

    pub fn save_shared_signer(
        &self,
        shared_signer_id: EventId,
        owner_public_key: XOnlyPublicKey,
        shared_signer: SharedSigner,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn
            .prepare_cached("INSERT OR IGNORE INTO shared_signers (shared_signer_id, owner_public_key, shared_signer) VALUES (?, ?, ?);")?;
        stmt.execute((
            shared_signer_id.to_hex(),
            owner_public_key.to_string(),
            shared_signer.encrypt_with_keys(&self.keys)?,
        ))?;
        tracing::info!("Saved shared signer {shared_signer_id}");
        Ok(())
    }

    pub fn get_public_key_for_my_shared_signer(
        &self,
        shared_signer_id: EventId,
    ) -> Result<XOnlyPublicKey, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT public_key FROM my_shared_signers WHERE shared_signer_id = ? LIMIT 1;",
        )?;
        let mut rows = stmt.query([shared_signer_id.to_hex()])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("my shared signer".into()))?;
        let public_key: String = row.get(0)?;
        Ok(XOnlyPublicKey::from_str(&public_key)?)
    }

    pub fn get_my_shared_signers(&self) -> Result<BTreeMap<EventId, XOnlyPublicKey>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare("SELECT shared_signer_id, public_key FROM my_shared_signers;")?;
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
    }

    pub fn get_my_shared_signers_by_signer_id(
        &self,
        signer_id: EventId,
    ) -> Result<BTreeMap<EventId, XOnlyPublicKey>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
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
    }

    pub fn get_owner_public_key_for_shared_signer(
        &self,
        shared_signer_id: EventId,
    ) -> Result<XOnlyPublicKey, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT owner_public_key FROM shared_signers WHERE shared_signer_id = ? LIMIT 1;",
        )?;
        let mut rows = stmt.query([shared_signer_id.to_hex()])?;
        let row = rows
            .next()?
            .ok_or(Error::NotFound("shared signer".into()))?;
        let public_key: String = row.get(0)?;
        Ok(XOnlyPublicKey::from_str(&public_key)?)
    }

    pub fn get_shared_signers(&self) -> Result<Vec<GetSharedSigner>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT shared_signer_id, owner_public_key, shared_signer FROM shared_signers;",
        )?;
        let mut rows = stmt.query([])?;
        let mut list = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let shared_signer_id: String = row.get(0)?;
            let public_key: String = row.get(1)?;
            let shared_signer: String = row.get(2)?;
            list.push(GetSharedSigner {
                shared_signer_id: EventId::from_hex(shared_signer_id)?,
                owner_public_key: XOnlyPublicKey::from_str(&public_key)?,
                shared_signer: SharedSigner::decrypt_with_keys(&self.keys, shared_signer)?,
            });
        }
        Ok(list)
    }

    pub fn get_shared_signers_by_public_key(
        &self,
        public_key: XOnlyPublicKey,
    ) -> Result<Vec<GetSharedSigner>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare(
            "SELECT shared_signer_id, shared_signer FROM shared_signers WHERE owner_public_key = ?;",
        )?;
        let mut rows = stmt.query([public_key.to_string()])?;
        let mut list = Vec::new();
        while let Ok(Some(row)) = rows.next() {
            let shared_signer_id: String = row.get(0)?;
            let shared_signer: String = row.get(1)?;
            list.push(GetSharedSigner {
                shared_signer_id: EventId::from_hex(shared_signer_id)?,
                owner_public_key: public_key,
                shared_signer: SharedSigner::decrypt_with_keys(&self.keys, shared_signer)?,
            });
        }
        Ok(list)
    }
}
