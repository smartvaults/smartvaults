// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;

use nostr_sdk::{PublicKey, Timestamp};
use smartvaults_protocol::v1::{Label, SharedSigner};
use smartvaults_protocol::v2::{Approval, Vault, VaultIdentifier};

#[derive(Debug, Clone)]
pub(crate) struct InternalVault {
    pub vault: Vault,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalApproval {
    pub public_key: PublicKey,
    pub approval: Approval,
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
pub(crate) struct InternalSharedSigner {
    pub owner_public_key: PublicKey,
    pub shared_signer: SharedSigner,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalLabel {
    pub vault_id: VaultIdentifier,
    pub label: Label,
}
