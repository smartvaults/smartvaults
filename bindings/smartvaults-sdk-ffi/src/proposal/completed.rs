// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use nostr_ffi::{EventId, Timestamp};
use smartvaults_sdk::core::proposal;
use smartvaults_sdk::types;
use uniffi::{Enum, Object};

use super::Period;

#[derive(Enum)]
pub enum CompletedProposal {
    Spending {
        txid: String,
        description: String,
    },
    KeyAgentPayment {
        txid: String,
        signer_descriptor: String,
        description: String,
        period: Period,
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
            proposal::CompletedProposal::KeyAgentPayment {
                tx,
                signer_descriptor,
                description,
                period,
            } => Self::KeyAgentPayment {
                txid: tx.txid().to_string(),
                signer_descriptor: signer_descriptor.to_string(),
                description,
                period: Period {
                    from: Timestamp::from_secs(period.from),
                    to: Timestamp::from_secs(period.to),
                },
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

#[derive(Clone, Object)]
pub struct GetCompletedProposal {
    inner: types::GetCompletedProposal,
}

impl From<types::GetCompletedProposal> for GetCompletedProposal {
    fn from(inner: types::GetCompletedProposal) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl GetCompletedProposal {
    pub fn completed_proposal_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.completed_proposal_id.into())
    }

    pub fn policy_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.policy_id.into())
    }

    pub fn completed_proposal(&self) -> CompletedProposal {
        self.inner.proposal.clone().into()
    }
}
