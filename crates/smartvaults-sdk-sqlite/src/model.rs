// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::signer::SharedSigner;
use smartvaults_protocol::nostr::nips::nip46::Message;
use smartvaults_protocol::nostr::{EventId, Timestamp};

#[derive(Debug, Clone)]
pub struct GetSharedSignerRaw {
    pub shared_signer_id: EventId,
    pub owner_public_key: XOnlyPublicKey,
    pub shared_signer: SharedSigner,
}

#[derive(Debug, Clone)]
pub struct NostrConnectRequest {
    pub event_id: EventId,
    pub app_public_key: XOnlyPublicKey,
    pub message: Message,
    pub timestamp: Timestamp,
    pub approved: bool,
}
