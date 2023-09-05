// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::policy;
use coinstr_protocol::v1::util::serde::Error as SerdeError;
use deadpool_sqlite::{CreatePoolError, InteractError, PoolError};
use nostr_sdk::event;
use nostr_sdk::event::id;

use super::encryption::Error as EncryptionError;
use super::migration::MigrationError;

/// Store error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Sqlite error
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    /// Pool error
    #[error(transparent)]
    CreateDeadPool(#[from] CreatePoolError),
    /// Pool error
    #[error(transparent)]
    DeadPool(#[from] PoolError),
    /// Pool error
    #[error(transparent)]
    DeadPoolInteract(#[from] InteractError),
    /// Migration error
    #[error(transparent)]
    Migration(#[from] MigrationError),
    /// Encryption error
    #[error(transparent)]
    Encryption(#[from] EncryptionError),
    /// Keys error
    #[error(transparent)]
    Keys(#[from] nostr_sdk::nostr::key::Error),
    /// EventId error
    #[error(transparent)]
    EventId(#[from] id::Error),
    /// Event error
    #[error(transparent)]
    Event(#[from] event::Error),
    /// Metadata error
    #[error(transparent)]
    Metadata(#[from] nostr_sdk::types::metadata::Error),
    /// NIP46 error
    #[error(transparent)]
    NIP46(#[from] nostr_sdk::nips::nip46::Error),
    /// JSON error
    #[error(transparent)]
    JSON(#[from] SerdeError),
    /// Secp256k1 error
    #[error(transparent)]
    Secp256k1(#[from] nostr_sdk::secp256k1::Error),
    /// Hash error
    #[error(transparent)]
    Hash(#[from] coinstr_core::bitcoin::hashes::hex::Error),
    /// Policy error
    #[error(transparent)]
    Policy(#[from] policy::Error),
    #[error(transparent)]
    Join(#[from] tokio::task::JoinError),
    /// Label error
    #[error(transparent)]
    Label(#[from] coinstr_protocol::v1::label::Error),
    /// Not found
    #[error("sqlite: {0} not found")]
    NotFound(String),
}
