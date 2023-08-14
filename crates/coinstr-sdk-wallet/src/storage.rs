// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use bdk::chain::{Append, PersistBackend};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sled::Tree;
use thiserror::Error;

const KEYCHAIN_STORE_KEY: &str = "bdk_keychain";

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Sled(#[from] sled::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct CoinstrWalletStorage {
    tree: Tree,
}

impl CoinstrWalletStorage {
    pub fn new(tree: Tree) -> Self {
        Self { tree }
    }
}

impl<K> PersistBackend<K> for CoinstrWalletStorage
where
    K: Default + Clone + Append + Serialize + DeserializeOwned,
{
    type WriteError = Error;
    type LoadError = Error;

    fn write_changes(&mut self, changeset: &K) -> Result<(), Self::WriteError> {
        if changeset.is_empty() {
            return Ok(());
        }

        match self.tree.get(KEYCHAIN_STORE_KEY)? {
            Some(keychain_store) => {
                let mut keychain_store: K = serde_json::from_slice(&keychain_store)?;
                keychain_store.append(changeset.clone());
                self.tree
                    .insert(KEYCHAIN_STORE_KEY, serde_json::to_vec(&keychain_store)?)?
            }
            None => self
                .tree
                .insert(KEYCHAIN_STORE_KEY, serde_json::to_vec(changeset)?)?,
        };

        Ok(())
    }

    fn load_from_persistence(&mut self) -> Result<K, Self::LoadError> {
        if let Some(k) = self.tree.get(KEYCHAIN_STORE_KEY)? {
            let keychain_store: K = serde_json::from_slice(&k)?;
            Ok(keychain_store)
        } else {
            // If there is no keychain store, we return an empty one
            Ok(K::default())
        }
    }
}
