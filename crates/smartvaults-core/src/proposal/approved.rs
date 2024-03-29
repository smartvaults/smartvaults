// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use serde::{Deserialize, Serialize};

use super::ProposalType;
use crate::util::{deserialize_psbt, serialize_psbt};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ApprovedProposal {
    Spending {
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
    ProofOfReserve {
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
    KeyAgentPayment {
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
}

impl ApprovedProposal {
    pub fn spending(psbt: PartiallySignedTransaction) -> Self {
        Self::Spending { psbt }
    }

    pub fn proof_of_reserve(psbt: PartiallySignedTransaction) -> Self {
        Self::ProofOfReserve { psbt }
    }

    pub fn key_agent_payment(psbt: PartiallySignedTransaction) -> Self {
        Self::KeyAgentPayment { psbt }
    }

    pub fn get_type(&self) -> ProposalType {
        match self {
            Self::Spending { .. } => ProposalType::Spending,
            Self::ProofOfReserve { .. } => ProposalType::ProofOfReserve,
            Self::KeyAgentPayment { .. } => ProposalType::KeyAgentPayment,
        }
    }

    pub fn psbt(&self) -> PartiallySignedTransaction {
        match self {
            Self::Spending { psbt, .. } => psbt.clone(),
            Self::ProofOfReserve { psbt, .. } => psbt.clone(),
            Self::KeyAgentPayment { psbt } => psbt.clone(),
        }
    }
}
