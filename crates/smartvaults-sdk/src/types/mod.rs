// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashSet;
use std::ops::Deref;

use nostr_sdk::{EventId, Profile, Timestamp};
use smartvaults_core::bdk::wallet::Balance;
use smartvaults_core::bdk::LocalUtxo;
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::Address;
use smartvaults_core::{ApprovedProposal, Policy, SharedSigner};
use smartvaults_protocol::v1::SignerOffering;
pub use smartvaults_sdk_sqlite::model::*;

pub mod backup;

pub use self::backup::PolicyBackup;
use crate::manager::TransactionDetails;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GetPolicy {
    pub policy_id: EventId,
    pub policy: Policy,
    pub balance: Balance,
    pub last_sync: Option<Timestamp>,
}

impl Deref for GetPolicy {
    type Target = Policy;
    fn deref(&self) -> &Self::Target {
        &self.policy
    }
}

#[derive(Debug, Clone)]
pub struct GetApproval {
    pub approval_id: EventId,
    pub user: Profile,
    pub approved_proposal: ApprovedProposal,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub struct GetSharedSigner {
    pub shared_signer_id: EventId,
    pub owner: Profile,
    pub shared_signer: SharedSigner,
}

#[derive(Debug, Clone)]
pub struct GetUtxo {
    pub utxo: LocalUtxo,
    pub label: Option<String>,
    pub frozen: bool,
}

impl Deref for GetUtxo {
    type Target = LocalUtxo;
    fn deref(&self) -> &Self::Target {
        &self.utxo
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
    pub my: Vec<GetSigner>,
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
    pub signer: GetSigner,
    pub offering: SignerOffering,
}
