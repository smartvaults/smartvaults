// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use nostr_ffi::{EventId, Timestamp};
use nostr_sdk_ffi::profile::Profile;
use smartvaults_sdk::core::proposal;
use smartvaults_sdk::types;
use uniffi::{Enum, Object};

#[derive(Clone, Enum)]
pub enum ApprovedProposal {
    Spending { psbt: String },
    KeyAgentPayment { psbt: String },
    ProofOfReserve { psbt: String },
}

impl From<proposal::ApprovedProposal> for ApprovedProposal {
    fn from(value: proposal::ApprovedProposal) -> Self {
        match value {
            proposal::ApprovedProposal::Spending { psbt } => Self::Spending {
                psbt: psbt.to_string(),
            },
            proposal::ApprovedProposal::KeyAgentPayment { psbt } => Self::KeyAgentPayment {
                psbt: psbt.to_string(),
            },
            proposal::ApprovedProposal::ProofOfReserve { psbt } => Self::ProofOfReserve {
                psbt: psbt.to_string(),
            },
        }
    }
}

#[derive(Object)]
pub struct GetApproval {
    inner: types::GetApproval,
}

impl From<types::GetApproval> for GetApproval {
    fn from(inner: types::GetApproval) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl GetApproval {
    pub fn approval_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.approval_id.into())
    }

    pub fn user(&self) -> Arc<Profile> {
        Arc::new(self.inner.user.clone().into())
    }

    pub fn approved_proposal(&self) -> ApprovedProposal {
        self.inner.approved_proposal.clone().into()
    }

    pub fn timestamp(&self) -> Arc<Timestamp> {
        Arc::new(self.inner.timestamp.into())
    }
}
