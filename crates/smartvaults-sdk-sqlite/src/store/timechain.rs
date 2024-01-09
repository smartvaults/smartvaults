// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use smartvaults_core::bdk::wallet::ChangeSet;
use smartvaults_core::hashes::sha256::Hash as Sha256Hash;

use super::{Error, Store, StoreEncryption};

impl Store {
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn save_changeset(
        &self,
        descriptor_hash: Sha256Hash,
        changeset: ChangeSet,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let data: Vec<u8> = changeset.encrypt(&cipher)?;
            conn.execute(
                "INSERT INTO timechain (descriptor_hash, data) VALUES (?, ?) ON CONFLICT(descriptor_hash) DO UPDATE SET data = ?;",
                (
                    descriptor_hash.to_string(),
                    data.clone(),
                    data,
                ),
            )?;
            Ok(())
        }).await?
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_changeset(&self, descriptor_hash: Sha256Hash) -> Result<ChangeSet, Error> {
        let conn = self.acquire().await?;
        let cipher = self.cipher.clone();
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT data FROM timechain WHERE descriptor_hash = ?;")?;
            let mut rows = stmt.query([descriptor_hash.to_string()])?;
            let row = rows.next()?.ok_or(Error::NotFound("changeset".into()))?;
            let data: Vec<u8> = row.get(0)?;
            Ok(ChangeSet::decrypt(&cipher, data)?)
        })
        .await?
    }
}
