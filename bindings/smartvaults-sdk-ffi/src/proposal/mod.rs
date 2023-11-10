// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use nostr_sdk_ffi::{EventId, Timestamp};
use smartvaults_sdk::core::proposal;
use smartvaults_sdk::types;

mod approved;
mod completed;

pub use self::approved::{ApprovedProposal, GetApproval};
pub use self::completed::{CompletedProposal, GetCompletedProposal};

pub struct Period {
    pub from: Arc<Timestamp>,
    pub to: Arc<Timestamp>,
}

impl From<Period> for proposal::Period {
    fn from(value: Period) -> Self {
        Self {
            from: value.from.as_u64(),
            to: value.to.as_u64(),
        }
    }
}

pub enum Proposal {
    Spending {
        descriptor: String,
        to_address: String,
        amount: u64,
        description: String,
        psbt: String,
    },
    KeyAgentPayment {
        descriptor: String,
        signer_descriptor: String,
        amount: u64,
        description: String,
        period: Period,
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
            proposal::Proposal::KeyAgentPayment {
                descriptor,
                signer_descriptor,
                amount,
                description,
                period,
                psbt,
            } => Self::KeyAgentPayment {
                descriptor: descriptor.to_string(),
                signer_descriptor: signer_descriptor.to_string(),
                amount,
                description,
                period: Period {
                    from: Arc::new(Timestamp::from_secs(period.from)),
                    to: Arc::new(Timestamp::from_secs(period.to)),
                },
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
