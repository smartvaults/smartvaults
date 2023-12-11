// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::EventId;
use smartvaults_core::bitcoin::Txid;
use smartvaults_core::secp256k1::XOnlyPublicKey;

pub(crate) mod dir;
pub mod format;

/// Get the first 8 chars of an [`EventId`]
pub fn cut_event_id(event_id: EventId) -> String {
    event_id.to_string()[..8].to_string()
}

/// Get the first 8 chars of a [`XOnlyPublicKey`]
pub fn cut_public_key(pk: XOnlyPublicKey) -> String {
    let pk = pk.to_string();
    format!("{}:{}", &pk[0..8], &pk[pk.len() - 8..])
}

/// Get the first 8 chars of an [`Txid`]
pub fn cut_txid(txid: Txid) -> String {
    txid.to_string()[..8].to_string()
}
