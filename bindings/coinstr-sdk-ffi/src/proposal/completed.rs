// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::proposal;
use coinstr_sdk::db::model;

#[derive(Clone)]
pub enum CompletedProposal {
    Spending {
        txid: String,
        description: String,
    },
    ProofOfReserve {
        descriptor: String,
        message: String,
        psbt: String,
    },
}

impl From<proposal::CompletedProposal> for CompletedProposal {
    fn from(value: proposal::CompletedProposal) -> Self {
        match value {
            proposal::CompletedProposal::Spending { description, tx } => Self::Spending {
                txid: tx.txid().to_string(),
                description,
            },
            proposal::CompletedProposal::ProofOfReserve {
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
pub struct GetCompletedProposal {
    inner: model::GetCompletedProposal,
}

impl From<model::GetCompletedProposal> for GetCompletedProposal {
    fn from(inner: model::GetCompletedProposal) -> Self {
        Self { inner }
    }
}

impl GetCompletedProposal {
    pub fn completed_proposal_id(&self) -> String {
        self.inner.completed_proposal_id.to_string()
    }

    pub fn policy_id(&self) -> String {
        self.inner.policy_id.to_string()
    }

    pub fn completed_proposal(&self) -> CompletedProposal {
        self.inner.proposal.clone().into()
    }
}
