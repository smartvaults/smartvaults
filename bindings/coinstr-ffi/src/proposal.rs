// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::proposal;

#[derive(Clone)]
pub enum Proposal {
    Spending {
        to_address: String,
        amount: u64,
        description: String,
        psbt: String,
    },
    ProofOfReserve {
        message: String,
        psbt: String,
    },
}

impl From<proposal::Proposal> for Proposal {
    fn from(value: proposal::Proposal) -> Self {
        match value {
            proposal::Proposal::Spending {
                to_address,
                amount,
                description,
                psbt,
            } => Self::Spending {
                to_address: to_address.to_string(),
                amount,
                description,
                psbt: psbt.to_string(),
            },
            proposal::Proposal::ProofOfReserve { message, psbt } => Self::ProofOfReserve {
                message,
                psbt: psbt.to_string(),
            },
        }
    }
}
