// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::proposal;

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
