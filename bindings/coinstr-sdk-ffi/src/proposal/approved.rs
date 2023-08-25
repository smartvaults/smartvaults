// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::core::proposal;
use coinstr_sdk::db::model;
use nostr_sdk_ffi::{EventId, PublicKey, Timestamp};

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
    inner: model::GetApproval,
}

impl From<model::GetApproval> for GetApproval {
    fn from(inner: model::GetApproval) -> Self {
        Self { inner }
    }
}

impl GetApproval {
    pub fn approval_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.approval_id.into())
    }

    pub fn public_key(&self) -> Arc<PublicKey> {
        Arc::new(self.inner.public_key.into())
    }

    pub fn approved_proposal(&self) -> ApprovedProposal {
        self.inner.approved_proposal.clone().into()
    }

    pub fn timestamp(&self) -> Arc<Timestamp> {
        Arc::new(self.inner.timestamp.into())
    }
}
