// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

// Use single parametrized replaceable event for proposals (both pending and completed).
// When a pending proposal it's finalized, replace the it with the completed

use smartvaults_core::bitcoin::Address;
use smartvaults_core::miniscript::Descriptor;

use super::NetworkMagic;

pub struct Recipient {
    pub address: Address,
    pub amount: u64,
}

pub enum Proposal {
    Pending(PendingProposal),
    Completed(CompletedProposal),
}

pub enum PendingProposal {
    Spending {
        descriptor: Descriptor<String>,
        addresses: Vec<Recipient>,
        description: String,
        network: NetworkMagic,
    },
}

pub enum CompletedProposal {
    Spending {},
}
