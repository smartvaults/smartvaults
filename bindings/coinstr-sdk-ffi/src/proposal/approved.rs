// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::proposal;

#[derive(Clone)]
pub enum ApprovedProposal {
    Spending { psbt: String },
    ProofOfReserve { psbt: String },
}

impl From<proposal::ApprovedProposal> for ApprovedProposal {
    fn from(value: proposal::ApprovedProposal) -> Self {
        match value {
            proposal::ApprovedProposal::Spending { psbt } => Self::Spending {
                psbt: psbt.to_string(),
            },
            proposal::ApprovedProposal::ProofOfReserve { psbt } => Self::ProofOfReserve {
                psbt: psbt.to_string(),
            },
        }
    }
}
