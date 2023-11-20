// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Proposals

use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::{Address, Network};
use smartvaults_core::miniscript::Descriptor;

mod proto;

/// Address recipient
pub struct Recipient {
    /// Address
    pub address: Address,
    /// Amount in SAT
    pub amount: u64,
}

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
