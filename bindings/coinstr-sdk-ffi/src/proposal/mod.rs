// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::proposal;

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
