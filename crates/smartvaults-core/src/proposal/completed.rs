// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::Transaction;
use keechain_core::miniscript::Descriptor;
use serde::{Deserialize, Serialize};

use crate::util::{deserialize_psbt, serialize_psbt};

use super::ProposalType;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CompletedProposal {
    Spending {
        tx: Transaction,
        description: String,
    },
    ProofOfReserve {
        message: String,
        descriptor: Descriptor<String>,
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
    KeyAgentPayment {
        tx: Transaction,
        signer_descriptor: Descriptor<String>,
        description: String,
    },
}

impl CompletedProposal {
    pub fn spending<S>(tx: Transaction, description: S) -> Self
    where
        S: Into<String>,
    {
        Self::Spending {
            tx,
            description: description.into(),
        }
    }

    pub fn proof_of_reserve<S>(
        message: S,
        descriptor: Descriptor<String>,
        psbt: PartiallySignedTransaction,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::ProofOfReserve {
            message: message.into(),
            descriptor,
            psbt,
        }
    }

    pub fn key_agent_payment<S>(
        tx: Transaction,
        signer_descriptor: Descriptor<String>,
        description: S,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::KeyAgentPayment {
            tx,
            signer_descriptor,
            description: description.into(),
        }
    }

    pub fn get_type(&self) -> ProposalType {
        match self {
            Self::Spending { .. } => ProposalType::Spending,
            Self::ProofOfReserve { .. } => ProposalType::ProofOfReserve,
            Self::KeyAgentPayment { .. } => ProposalType::KeyAgentPayment,
        }
    }

    pub fn tx(&self) -> Option<Transaction> {
        match self {
            Self::Spending { tx, .. } => Some(tx.clone()),
            Self::KeyAgentPayment { tx, .. } => Some(tx.clone()),
            _ => None,
        }
    }

    pub fn desc(&self) -> String {
        match self {
            Self::Spending { description, .. } => description.clone(),
            Self::ProofOfReserve { message, .. } => message.clone(),
            Self::KeyAgentPayment { description, .. } => description.clone(),
        }
    }

    pub fn export_proof(&self) -> Option<String> {
        match self {
            Self::ProofOfReserve {
                message,
                descriptor,
                psbt,
                ..
            } => {
                let json = serde_json::json!({
                    "message": message,
                    "descriptor": descriptor.to_string(),
                    "psbt": psbt.to_string()
                });
                Some(json.to_string())
            }
            _ => None,
        }
    }
}
