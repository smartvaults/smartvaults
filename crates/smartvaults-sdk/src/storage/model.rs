// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;

use nostr_sdk::{EventId, Timestamp};
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::{Policy, Proposal};

#[derive(Debug, Clone)]
pub(crate) struct InternalPolicy {
    pub policy: Policy,
    pub public_keys: Vec<XOnlyPublicKey>,
    pub last_sync: Option<Timestamp>,
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
