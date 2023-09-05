// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::cmp::Ordering;
use std::ops::Deref;

use coinstr_core::bdk::wallet::Balance;
use coinstr_core::bdk::LocalUtxo;
use coinstr_core::bitcoin::address::NetworkUnchecked;
use coinstr_core::bitcoin::Address;
use coinstr_core::secp256k1::XOnlyPublicKey;
use coinstr_core::signer::{SharedSigner, Signer};
use coinstr_core::{ApprovedProposal, CompletedProposal, Policy, Proposal};
use nostr_sdk::nips::nip46::Message;
use nostr_sdk::{EventId, Timestamp};

use crate::manager::wallet::TransactionDetails;

#[derive(PartialEq, Eq)]
pub(crate) struct InternalGetPolicy {
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

#[derive(Debug, Clone)]
pub struct GetPolicy {
    pub policy_id: EventId,
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
pub struct GetApproval {
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

#[derive(Debug, Clone)]
pub struct GetSharedSigner {
    pub shared_signer_id: EventId,
    pub owner_public_key: XOnlyPublicKey,
    pub shared_signer: SharedSigner,
}

#[derive(Debug, Clone, Default)]
pub struct GetAllSigners {
    pub my: Vec<GetSigner>,
    pub contacts: Vec<GetSharedSigner>,
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
    pub signed: bool,
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
    pub frozen: bool,
}

#[derive(Debug, Clone)]
pub struct GetAddress {
    pub address: Address<NetworkUnchecked>,
    pub label: Option<String>,
}

impl Deref for GetAddress {
    type Target = Address<NetworkUnchecked>;
    fn deref(&self) -> &Self::Target {
        &self.address
    }
}

#[derive(Debug, Clone)]
pub struct GetTransaction {
    pub policy_id: EventId,
    pub tx: TransactionDetails,
    pub label: Option<String>,
    pub block_explorer: Option<String>,
}

impl Deref for GetTransaction {
    type Target = TransactionDetails;
    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}
