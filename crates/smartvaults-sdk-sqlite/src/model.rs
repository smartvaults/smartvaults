// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;
use std::ops::Deref;

use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::signer::{SharedSigner, Signer};
use smartvaults_core::{ApprovedProposal, CompletedProposal, Policy, Proposal};
use smartvaults_protocol::nostr::nips::nip46::Message;
use smartvaults_protocol::nostr::{EventId, Timestamp};

#[derive(PartialEq, Eq)]
pub struct InternalGetPolicy {
    pub policy_id: EventId,
    pub policy: Policy,
    pub last_sync: Option<Timestamp>,
}

impl Ord for InternalGetPolicy {
    fn cmp(&self, other: &Self) -> Ordering {
        self.policy.cmp(&other.policy)
    }
}

impl PartialOrd for InternalGetPolicy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub struct GetApprovedProposals {
    pub policy_id: EventId,
    pub proposal: Proposal,
    pub approved_proposals: Vec<ApprovedProposal>,
}

#[derive(Debug, Clone)]
pub struct GetApprovalRaw {
    pub approval_id: EventId,
    pub public_key: XOnlyPublicKey,
    pub approved_proposal: ApprovedProposal,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub struct GetSigner {
    pub signer_id: EventId,
    pub signer: Signer,
}

impl Deref for GetSigner {
    type Target = Signer;
    fn deref(&self) -> &Self::Target {
        &self.signer
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetProposal {
    pub proposal_id: EventId,
    pub policy_id: EventId,
    pub proposal: Proposal,
    pub signed: bool,
}

impl PartialOrd for GetProposal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GetProposal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.proposal.cmp(&other.proposal)
    }
}

#[derive(Debug, Clone)]
pub struct GetCompletedProposal {
    pub policy_id: EventId,
    pub completed_proposal_id: EventId,
    pub proposal: CompletedProposal,
}
