// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use deadpool_sqlite::{CreatePoolError, InteractError, PoolError};
use smartvaults_core::{bitcoin, policy, secp256k1};
use smartvaults_protocol::nostr::event::id;
use smartvaults_protocol::nostr::{self, event};
use smartvaults_protocol::v1::util::serde::Error as SerdeError;

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
    Keys(#[from] nostr::key::Error),
    /// EventId error
    #[error(transparent)]
    EventId(#[from] id::Error),
    /// Event error
    #[error(transparent)]
    Event(#[from] event::Error),
    /// Metadata error
    #[error(transparent)]
    Metadata(#[from] nostr::types::metadata::Error),
    /// NIP46 error
    #[error(transparent)]
    NIP46(#[from] nostr::nips::nip46::Error),
    /// Url error
    #[error(transparent)]
    Url(#[from] nostr::types::url::ParseError),
    /// JSON error
    #[error(transparent)]
    JSON(#[from] SerdeError),
    /// Secp256k1 error
    #[error(transparent)]
    Secp256k1(#[from] secp256k1::Error),
    /// Hash error
    #[error(transparent)]
    Hash(#[from] bitcoin::hashes::hex::Error),
    /// Policy error
    #[error(transparent)]
    Policy(#[from] policy::Error),
    /// Label error
    #[error(transparent)]
    Label(#[from] smartvaults_protocol::v1::label::Error),
    /// Not found
    #[error("sqlite: {0} not found")]
    NotFound(String),
}
