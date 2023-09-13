// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;

use nostr_sdk::{EventId, Metadata, Timestamp};
use smartvaults_core::bdk::wallet::Balance;
use smartvaults_core::bdk::LocalUtxo;
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::Address;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::{ApprovedProposal, Policy, SharedSigner};
pub use smartvaults_sdk_sqlite::model::*;

pub mod backup;

pub use self::backup::PolicyBackup;
use crate::{manager::TransactionDetails, util};

#[derive(Debug, Clone, Eq)]
pub struct User {
    public_key: XOnlyPublicKey,
    metadata: Metadata,
}

impl PartialEq for User {
    fn eq(&self, other: &Self) -> bool {
        self.public_key == other.public_key
    }
}

impl User {
    pub fn new(public_key: XOnlyPublicKey, metadata: Metadata) -> Self {
        Self {
            public_key,
            metadata,
        }
    }

    pub fn public_key(&self) -> XOnlyPublicKey {
        self.public_key
    }

    pub fn metadata(&self) -> Metadata {
        self.metadata.clone()
    }

    pub fn name(&self) -> String {
        let metadata: Metadata = self.metadata();
        if let Some(display_name) = metadata.display_name {
            display_name
        } else if let Some(name) = metadata.name {
            name
        } else {
            util::cut_public_key(self.public_key)
        }
    }
}

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
    pub user: User,
    pub approved_proposal: ApprovedProposal,
    pub timestamp: Timestamp,
}

#[derive(Debug, Clone)]
pub struct GetSharedSigner {
    pub shared_signer_id: EventId,
    pub owner: User,
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
