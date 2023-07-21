// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use bdk::bitcoin::XOnlyPublicKey;
use bdk::Balance;
use coinstr_core::signer::{SharedSigner, Signer};
use coinstr_core::{ApprovedProposal, Policy, Proposal};
use nostr_sdk::nips::nip46::Message;
use nostr_sdk::{EventId, Timestamp};

use crate::types::Notification;

#[derive(Debug, Clone)]
pub struct GetPolicy {
    pub policy_id: EventId,
    pub policy: Policy,
    pub last_sync: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct GetDetailedPolicyResult {
    pub policy: Policy,
    pub balance: Option<Balance>,
    pub last_sync: Option<Timestamp>,
}

pub struct GetApprovedProposals {
    pub policy_id: EventId,
    pub proposal: Proposal,
    pub approved_proposals: Vec<ApprovedProposal>,
}

#[derive(Debug, Clone)]
pub struct GetNotificationsResult {
    pub notification: Notification,
    pub timestamp: Timestamp,
    pub seen: bool,
}

#[derive(Debug, Clone)]
pub struct GetApprovedProposalResult {
    pub public_key: XOnlyPublicKey,
    pub approved_proposal: ApprovedProposal,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub struct GetSharedSignerResult {
    pub owner_public_key: XOnlyPublicKey,
    pub shared_signer: SharedSigner,
}

#[derive(Debug, Clone, Default)]
pub struct GetAllSigners {
    pub my: BTreeMap<EventId, Signer>,
    pub contacts: BTreeMap<EventId, GetSharedSignerResult>,
}

#[derive(Debug, Clone)]
pub struct NostrConnectRequest {
    pub app_public_key: XOnlyPublicKey,
    pub message: Message,
    pub timestamp: Timestamp,
    pub approved: bool,
}

#[derive(Debug, Clone)]
pub struct GetProposal {
    pub proposal_id: EventId,
    pub policy_id: EventId,
    pub proposal: Proposal,
}
