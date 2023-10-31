// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Proposals

// Use single parametrized replaceable event for proposals (both pending and completed).
// When a pending proposal it's finalized, replace the it with the completed

use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::Address;
use smartvaults_core::miniscript::Descriptor;

use super::NetworkMagic;

/// Address recipient
pub struct Recipient {
    /// Address
    pub address: Address,
    /// Amount in SAT
    pub amount: u64,
}

/// Proposal
pub struct Proposal {
    /// Status
    pub status: ProposalStatus,
    /// Network
    pub network: NetworkMagic,
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
}

/// Completed proposal
pub enum CompletedProposal {
    /// Spending
    Spending {},
}
