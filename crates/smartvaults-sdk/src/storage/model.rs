// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;
use std::ops::Deref;

use nostr_sdk::{EventId, PublicKey, Timestamp};
use smartvaults_protocol::v1::Label;
use smartvaults_protocol::v2::{Approval, Vault, VaultIdentifier, VaultMetadata};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InternalVault {
    pub event_id: EventId,
    pub vault: Vault,
    pub metadata: VaultMetadata,
}

impl Deref for InternalVault {
    type Target = Vault;

    fn deref(&self) -> &Self::Target {
        &self.vault
    }
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
pub(crate) struct InternalLabel {
    pub vault_id: VaultIdentifier,
    pub label: Label,
}
