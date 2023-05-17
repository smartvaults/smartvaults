// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use nostr_sdk::{prelude::TagKind, Event, EventId, Tag};

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
