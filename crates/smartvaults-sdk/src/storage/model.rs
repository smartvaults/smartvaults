// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;

use nostr_sdk::{EventId, Timestamp};
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::{ApprovedProposal, CompletedProposal, Policy, Proposal, SharedSigner};
use smartvaults_protocol::v1::Label;

#[derive(Debug, Clone)]
pub(crate) struct InternalPolicy {
    pub policy: Policy,
    pub public_keys: Vec<XOnlyPublicKey>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalProposal {
    pub policy_id: EventId,
    pub proposal: Proposal,
    pub timestamp: Timestamp,
}

impl PartialOrd for InternalProposal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InternalProposal {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.timestamp != other.timestamp {
            self.timestamp.cmp(&other.timestamp)
        } else {
            self.proposal.cmp(&other.proposal)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalApproval {
    pub proposal_id: EventId,
    pub policy_id: EventId,
    pub public_key: XOnlyPublicKey,
    pub approval: ApprovedProposal,
    pub timestamp: Timestamp,
}

impl PartialOrd for InternalApproval {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InternalApproval {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.timestamp != other.timestamp {
            self.timestamp.cmp(&other.timestamp)
        } else {
            self.public_key.cmp(&other.public_key)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalCompletedProposal {
    pub policy_id: EventId,
    pub proposal: CompletedProposal,
    pub timestamp: Timestamp,
}

impl PartialOrd for InternalCompletedProposal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for InternalCompletedProposal {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.timestamp != other.timestamp {
            self.timestamp.cmp(&other.timestamp)
        } else {
            self.policy_id.cmp(&other.policy_id)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalSharedSigner {
    pub owner_public_key: XOnlyPublicKey,
    pub shared_signer: SharedSigner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalLabel {
    pub policy_id: EventId,
    pub label: Label,
}
