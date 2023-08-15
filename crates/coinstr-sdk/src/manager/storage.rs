// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bdk::chain::{Append, PersistBackend};
use nostr_sdk::hashes::sha256::Hash as Sha256Hash;
use thiserror::Error;

use crate::db::Store;
use crate::util::encryption::EncryptionWithKeys;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Store(#[from] crate::db::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct CoinstrWalletStorage {
    descriptor_hash: Sha256Hash,
    db: Store,
}

impl CoinstrWalletStorage {
    pub fn new(descriptor_hash: Sha256Hash, db: Store) -> Self {
        Self {
            descriptor_hash,
            db,
        }
    }
}

impl<K> PersistBackend<K> for CoinstrWalletStorage
where
    K: Default + Clone + Append + EncryptionWithKeys,
{
    type WriteError = Error;
    type LoadError = Error;

    fn write_changes(&mut self, changeset: &K) -> Result<(), Self::WriteError> {
        if changeset.is_empty() {
            return Ok(());
        }

        match self.db.get_changeset::<K>(self.descriptor_hash).ok() {
            Some(mut keychain_store) => {
                keychain_store.append(changeset.clone());
                self.db
                    .save_changeset(self.descriptor_hash, &keychain_store)?
            }
            None => self.db.save_changeset(self.descriptor_hash, changeset)?,
        };

        Ok(())
    }

    fn load_from_persistence(&mut self) -> Result<K, Self::LoadError> {
        match self.db.get_changeset::<K>(self.descriptor_hash) {
            Ok(k) => Ok(k),
            Err(e) => {
                tracing::error!("Impossible to load changeset: {e}");
                Ok(K::default())
            }
        }
    }
}
