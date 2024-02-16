// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::{EventId, PublicKey};
use smartvaults_core::bitcoin::Txid;

pub(crate) mod dir;
pub mod format;

/// Get the first 8 chars of an [`EventId`]
pub fn cut_event_id(event_id: EventId) -> String {
    event_id.to_string()[..8].to_string()
}

/// Get the first 8 chars of a [`PublicKey`]
pub fn cut_public_key(pk: PublicKey) -> String {
    let pk = pk.to_string();
    format!("{}:{}", &pk[0..8], &pk[pk.len() - 8..])
}

/// Get the first 8 chars of an [`Txid`]
pub fn cut_txid(txid: Txid) -> String {
    txid.to_string()[..8].to_string()
}
