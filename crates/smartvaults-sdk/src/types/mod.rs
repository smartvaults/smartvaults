// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;
use std::collections::HashSet;
use std::ops::Deref;

use nostr_sdk::{EventId, Profile, Timestamp};
use smartvaults_core::bdk::wallet::Balance;
use smartvaults_core::bdk::LocalOutput;
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::Address;
use smartvaults_protocol::v1::SignerOffering;
use smartvaults_protocol::v2::{Approval, Proposal, SharedSigner, Signer, Vault, VaultIdentifier};
pub use smartvaults_sdk_sqlite::model::*;

pub mod backup;

pub use self::backup::PolicyBackup;
use crate::manager::TransactionDetails;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetPolicy {
    pub vault: Vault,
    pub balance: Balance,
    pub last_sync: Timestamp,
}

impl PartialOrd for GetPolicy {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GetPolicy {
    fn cmp(&self, other: &Self) -> Ordering {
        self.vault.cmp(&other.vault)
    }
}

impl Deref for GetPolicy {
    type Target = Vault;

    fn deref(&self) -> &Self::Target {
        &self.vault
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetProposal {
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
        self.proposal.cmp(&other.proposal).reverse()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetApproval {
    pub approval_id: EventId,
    pub user: Profile,
    pub approval: Approval,
    pub timestamp: Timestamp,
}

impl PartialOrd for GetApproval {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GetApproval {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.timestamp != other.timestamp {
            self.timestamp.cmp(&other.timestamp).reverse()
        } else {
            self.approval_id.cmp(&other.approval_id)
        }
    }
}

pub struct GetApprovedProposals {
    pub proposal: Proposal,
    pub approvals: Vec<Approval>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetSharedSigner {
    pub shared_signer_id: EventId,
    pub owner: Profile,
    pub shared_signer: SharedSigner,
}

impl PartialOrd for GetSharedSigner {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GetSharedSigner {
    fn cmp(&self, other: &Self) -> Ordering {
        self.shared_signer.cmp(&other.shared_signer)
    }
}

#[derive(Debug, Clone)]
pub struct GetUtxo {
    pub utxo: LocalOutput,
    pub label: Option<String>,
    pub frozen: bool,
}

impl Deref for GetUtxo {
    type Target = LocalOutput;

    fn deref(&self) -> &Self::Target {
        &self.utxo
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetTransaction {
    pub vault_id: VaultIdentifier,
    pub tx: TransactionDetails,
    pub label: Option<String>,
    pub block_explorer: Option<String>,
}

impl PartialOrd for GetTransaction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GetTransaction {
    fn cmp(&self, other: &Self) -> Ordering {
        self.tx.cmp(&other.tx)
    }
}

impl Deref for GetTransaction {
    type Target = TransactionDetails;

    fn deref(&self) -> &Self::Target {
        &self.tx
    }
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

#[derive(Debug, Clone, Default)]
pub struct GetAllSigners {
    pub my: Vec<Signer>,
    pub contacts: Vec<GetSharedSigner>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyAgent {
    pub user: Profile,
    pub list: HashSet<SignerOffering>,
    pub verified: bool,
    pub is_contact: bool,
}

impl PartialOrd for KeyAgent {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KeyAgent {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.user.cmp(&other.user)
    }
}

impl Deref for KeyAgent {
    type Target = Profile;

    fn deref(&self) -> &Self::Target {
        &self.user
    }
}

#[derive(Debug, Clone)]
pub struct GetSignerOffering {
    pub id: EventId, // TODO: remove?
    pub signer: Signer,
    pub offering: SignerOffering,
}
