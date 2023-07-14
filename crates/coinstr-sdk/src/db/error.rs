// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::policy;
use coinstr_core::util::serde::Error as SerdeError;
use nostr_sdk::event;
use nostr_sdk::event::id::{self, EventId};

use super::migration::MigrationError;
use crate::util::encryption::EncryptionWithKeysError;

/// Store error
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Sqlite error
    #[error(transparent)]
    Sqlite(#[from] rusqlite::Error),
    /// Sqlite Pool error
    #[error(transparent)]
    Pool(#[from] r2d2::Error),
    /// Migration error
    #[error(transparent)]
    Migration(#[from] MigrationError),
    /// Bdk error
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    /// Encryption error
    #[error(transparent)]
    EncryptionWithKeys(#[from] EncryptionWithKeysError),
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
    /// Policy error
    #[error(transparent)]
    Policy(#[from] policy::Error),
    #[error(transparent)]
    Electrum(#[from] bdk::electrum_client::Error),
    #[error("electrum client not initialized")]
    ElectrumClientNotInit,
    /// Not found
    #[error("impossible to open policy {0} db")]
    FailedToOpenPolicyDb(EventId),
    /// Not found
    #[error("sqlite: {0} not found")]
    NotFound(String),
    /// Wallet ot found
    #[error("wallet not found")]
    WalletNotFound,
}
