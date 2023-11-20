// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Proposals

use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::Descriptor;

mod recipient;

pub use self::recipient::Recipient;
use crate::v2::proto::proposal::{
    ProtoCompletedKeyAgentPayment, ProtoCompletedProofOfReserve, ProtoCompletedProposal,
    ProtoCompletedProposalEnum, ProtoCompletedSpending, ProtoPendingKeyAgentPayment,
    ProtoPendingProofOfReserve, ProtoPendingProposal, ProtoPendingProposalEnum,
    ProtoPendingSpending, ProtoProposal, ProtoProposalEnum, ProtoProposalStatus,
};

/// Proposal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProposalType {
    /// Spending
    Spending,
    /// Proof of reserve
    ProofOfReserve,
    /// Key Agent Payment
    KeyAgentPayment,
}

/// Proposal
pub struct Proposal {
    /// Status
    pub status: ProposalStatus,
    /// Network
    pub network: Network,
}

impl Proposal {
    /// Get [`ProposalType`]
    pub fn r#type(&self) -> ProposalType {
        match &self.status {
            ProposalStatus::Pending(p) => match p {
                PendingProposal::Spending { .. } => ProposalType::Spending,
                PendingProposal::ProofOfReserve { .. } => ProposalType::ProofOfReserve,
                PendingProposal::KeyAgentPayment { .. } => ProposalType::KeyAgentPayment,
            },
            ProposalStatus::Completed(p) => match p {
                CompletedProposal::Spending { .. } => ProposalType::Spending,
                CompletedProposal::ProofOfReserve { .. } => ProposalType::ProofOfReserve,
                CompletedProposal::KeyAgentPayment { .. } => ProposalType::KeyAgentPayment,
            },
        }
    }
}

/// Proposal status
pub enum ProposalStatus {
    /// Pending proposal
    Pending(PendingProposal),
    /// Completed proposal
    Completed(CompletedProposal),
}

/// Pending proposal
pub enum PendingProposal {
    /// Spending
    Spending {
        /// Descriptor
        descriptor: Descriptor<String>,
        /// Recipients
        addresses: Vec<Recipient>,
        /// Description/note
        description: String,
        /// PSBT
        psbt: PartiallySignedTransaction,
    },
    /// Proof of reserve
    ProofOfReserve {},
    /// Key Agent Payment
    KeyAgentPayment {
        /// Descriptor
        descriptor: Descriptor<String>,
        /// Signer descriptor
        ///
        /// Needed to indentify the Key Agent and the signer used
        signer_descriptor: Descriptor<String>,
        /// Recipient
        recipient: Recipient,
        /// Description/note
        description: String,
        //period: Period,
        /// PSBT
        psbt: PartiallySignedTransaction,
    },
}

/// Completed proposal
pub enum CompletedProposal {
    /// Spending
    Spending {},
    /// Proof of reserve
    ProofOfReserve {},
    /// Key Agent Payment
    KeyAgentPayment {},
}

/* Self {
    address: Address::from_str(&recipient.address)?,
    amount: recipient.amount,
} */

impl From<&PendingProposal> for ProtoPendingProposal {
    fn from(value: &PendingProposal) -> Self {
        Self {
            proposal: Some(match value {
                PendingProposal::Spending {
                    descriptor,
                    addresses,
                    description,
                    psbt,
                } => ProtoPendingProposalEnum::Spending(ProtoPendingSpending {
                    descriptor: descriptor.to_string(),
                    addresses: addresses.iter().map(|r| r.into()).collect(),
                    description: description.to_owned(),
                    psbt: psbt.to_string(),
                }),
                PendingProposal::ProofOfReserve {} => {
                    ProtoPendingProposalEnum::ProofOfReserve(ProtoPendingProofOfReserve {})
                }
                PendingProposal::KeyAgentPayment {
                    descriptor,
                    signer_descriptor,
                    recipient,
                    description,
                    psbt,
                } => ProtoPendingProposalEnum::KeyAgentPayment(ProtoPendingKeyAgentPayment {
                    descriptor: descriptor.to_string(),
                    signer_descriptor: signer_descriptor.to_string(),
                    recipient: Some(recipient.into()),
                    description: description.to_owned(),
                    psbt: psbt.to_string(),
                }),
            }),
        }
    }
}

impl From<&CompletedProposal> for ProtoCompletedProposal {
    fn from(value: &CompletedProposal) -> Self {
        Self {
            proposal: Some(match value {
                CompletedProposal::Spending {} => {
                    ProtoCompletedProposalEnum::Spending(ProtoCompletedSpending {})
                }
                CompletedProposal::ProofOfReserve {} => {
                    ProtoCompletedProposalEnum::ProofOfReserve(ProtoCompletedProofOfReserve {})
                }
                CompletedProposal::KeyAgentPayment { .. } => {
                    ProtoCompletedProposalEnum::KeyAgentPayment(ProtoCompletedKeyAgentPayment {})
                }
            }),
        }
    }
}

impl From<&ProposalStatus> for ProtoProposalEnum {
    fn from(value: &ProposalStatus) -> Self {
        match value {
            ProposalStatus::Pending(pending) => Self::Pending(pending.into()),
            ProposalStatus::Completed(completed) => Self::Completed(completed.into()),
        }
    }
}

impl From<&Proposal> for ProtoProposal {
    fn from(proposal: &Proposal) -> Self {
        ProtoProposal {
            status: Some(ProtoProposalStatus {
                proposal: Some((&proposal.status).into()),
            }),
            network: proposal.network.magic().to_bytes().to_vec(),
        }
    }
}
