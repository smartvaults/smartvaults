// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bdk::chain::Append;
use nostr_sdk::hashes::sha256::Hash as Sha256Hash;

use super::{Error, Store};
use crate::util::encryption::EncryptionWithKeys;

impl Store {
    #[tracing::instrument(skip_all, level = "trace")]
    pub fn save_changeset<K>(&self, descriptor_hash: Sha256Hash, changeset: &K) -> Result<(), Error>
    where
        K: Default + Clone + Append + EncryptionWithKeys,
    {
        let conn = self.pool.get()?;
        let data: String = changeset.encrypt_with_keys(&self.keys)?;
        conn.execute(
            "INSERT INTO timechain (descriptor_hash, data) VALUES (?, ?) ON CONFLICT(descriptor_hash) DO UPDATE SET data = ?;",
            (
                descriptor_hash.to_string(),
                data.clone(),
                data,
            ),
        )?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn get_changeset<K>(&self, descriptor_hash: Sha256Hash) -> Result<K, Error>
    where
        K: Default + Clone + Append + EncryptionWithKeys,
    {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT data FROM timechain WHERE descriptor_hash = ?;")?;
        let mut rows = stmt.query([descriptor_hash.to_string()])?;
        let row = rows.next()?.ok_or(Error::NotFound("changeset".into()))?;
        let data: String = row.get(0)?;
        Ok(K::decrypt_with_keys(&self.keys, data)?)
    }
}
