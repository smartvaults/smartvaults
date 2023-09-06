// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::core::proposal;
use coinstr_sdk::types;
use nostr_sdk_ffi::EventId;

mod approved;
mod completed;

pub use self::approved::{ApprovedProposal, GetApproval};
pub use self::completed::{CompletedProposal, GetCompletedProposal};

#[derive(Clone)]
pub enum Proposal {
    Spending {
        descriptor: String,
        to_address: String,
        amount: u64,
        description: String,
        psbt: String,
    },
    ProofOfReserve {
        descriptor: String,
        message: String,
        psbt: String,
    },
}

impl From<proposal::Proposal> for Proposal {
    fn from(value: proposal::Proposal) -> Self {
        match value {
            proposal::Proposal::Spending {
                descriptor,
                to_address,
                amount,
                description,
                psbt,
            } => Self::Spending {
                descriptor: descriptor.to_string(),
                to_address: to_address.assume_checked().to_string(),
                amount,
                description,
                psbt: psbt.to_string(),
            },
            proposal::Proposal::ProofOfReserve {
                descriptor,
                message,
                psbt,
            } => Self::ProofOfReserve {
                descriptor: descriptor.to_string(),
                message,
                psbt: psbt.to_string(),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetProposal {
    inner: types::GetProposal,
}

impl From<types::GetProposal> for GetProposal {
    fn from(inner: types::GetProposal) -> Self {
        Self { inner }
    }
}

impl GetProposal {
    pub fn proposal_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.proposal_id.into())
    }

    pub fn policy_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.policy_id.into())
    }

    pub fn proposal(&self) -> Proposal {
        self.inner.proposal.clone().into()
    }

    pub fn is_signed(&self) -> bool {
        self.inner.signed
    }
}
