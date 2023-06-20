// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use bdk::bitcoin::{Txid, XOnlyPublicKey};
use nostr_sdk::{Event, EventId, Tag, TagKind};

pub(crate) mod dir;
pub mod encryption;
pub mod format;

pub fn extract_first_event_id(event: &Event) -> Option<EventId> {
    for tag in event.tags.iter() {
        if let Tag::Event(event_id, ..) = tag {
            return Some(*event_id);
        }
    }
    None
}

pub fn extract_first_public_key(event: &Event) -> Option<XOnlyPublicKey> {
    for tag in event.tags.iter() {
        if let Tag::PubKey(public_key, ..) = tag {
            return Some(*public_key);
        }
    }
    None
}

pub fn extract_tags_by_kind(event: &Event, kind: TagKind) -> Vec<&Tag> {
    let mut tags = Vec::new();
    for tag in event.tags.iter() {
        if kind == tag.kind() {
            tags.push(tag);
        }
    }
    tags
}

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
