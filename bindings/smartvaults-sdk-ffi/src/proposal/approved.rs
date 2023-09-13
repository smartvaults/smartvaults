// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use nostr_sdk_ffi::{EventId, Timestamp};
use smartvaults_sdk::core::proposal;
use smartvaults_sdk::types;

use crate::User;

#[derive(Clone)]
pub enum ApprovedProposal {
    Spending { psbt: String },
    ProofOfReserve { psbt: String },
}

impl From<proposal::ApprovedProposal> for ApprovedProposal {
    fn from(value: proposal::ApprovedProposal) -> Self {
        match value {
            proposal::ApprovedProposal::Spending { psbt } => Self::Spending {
                psbt: psbt.to_string(),
            },
            proposal::ApprovedProposal::ProofOfReserve { psbt } => Self::ProofOfReserve {
                psbt: psbt.to_string(),
            },
        }
    }
}

pub struct GetApproval {
    inner: types::GetApproval,
}

impl From<types::GetApproval> for GetApproval {
    fn from(inner: types::GetApproval) -> Self {
        Self { inner }
    }
}

impl GetApproval {
    pub fn approval_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.approval_id.into())
    }

    pub fn user(&self) -> Arc<User> {
        Arc::new(self.inner.user.clone().into())
    }

    pub fn approved_proposal(&self) -> ApprovedProposal {
        self.inner.approved_proposal.clone().into()
    }

    pub fn timestamp(&self) -> Arc<Timestamp> {
        Arc::new(self.inner.timestamp.into())
    }
}
