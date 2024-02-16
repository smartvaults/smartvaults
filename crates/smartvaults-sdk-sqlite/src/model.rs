// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use smartvaults_protocol::nostr::nips::nip46::Message;
use smartvaults_protocol::nostr::{EventId, PublicKey, Timestamp};

#[derive(Debug, Clone)]
pub struct NostrConnectRequest {
    pub event_id: EventId,
    pub app_public_key: PublicKey,
    pub message: Message,
    pub timestamp: Timestamp,
    pub approved: bool,
}
