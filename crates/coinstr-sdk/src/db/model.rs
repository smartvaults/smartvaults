// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::ops::Deref;

use bdk::bitcoin::{Address, XOnlyPublicKey};
use bdk::{Balance, LocalUtxo, TransactionDetails};
use coinstr_core::signer::{SharedSigner, Signer};
use coinstr_core::{ApprovedProposal, CompletedProposal, Policy, Proposal};
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
    pub event_id: EventId,
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

#[derive(Debug, Clone)]
pub struct GetCompletedProposal {
    pub policy_id: EventId,
    pub completed_proposal_id: EventId,
    pub proposal: CompletedProposal,
}

#[derive(Debug, Clone)]
pub struct GetUtxo {
    pub utxo: LocalUtxo,
    pub label: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GetAddress {
    pub address: Address,
    pub label: Option<String>,
}

impl Deref for GetAddress {
    type Target = Address;
    fn deref(&self) -> &Self::Target {
        &self.address
    }
}

#[derive(Debug, Clone)]
pub struct GetTransaction {
    pub policy_id: EventId,
    pub tx: TransactionDetails,
    pub label: Option<String>,
}

impl Deref for GetTransaction {
    type Target = TransactionDetails;
    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}
