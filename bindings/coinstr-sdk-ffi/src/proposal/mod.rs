// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::proposal;
use coinstr_sdk::db::model;

mod approved;
mod completed;

pub use self::approved::ApprovedProposal;
pub use self::completed::CompletedProposal;

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
                to_address: to_address.to_string(),
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
    inner: model::GetProposal
}

impl From<model::GetProposal> for GetProposal {
    fn from(inner: model::GetProposal) -> Self {
        Self { inner }
    }
}

impl GetProposal {
    pub fn proposal_id(&self) -> String {
        self.inner.proposal_id.to_string()
    }

    pub fn policy_id(&self) -> String {
        self.inner.policy_id.to_string()
    }

    pub fn proposal(&self) -> Proposal {
        self.inner.proposal.clone().into()
    }
}
