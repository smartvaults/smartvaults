// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::Transaction;
use keechain_core::miniscript::Descriptor;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompletedProposal {
    Spending {
        tx: Transaction,
    },
    ProofOfReserve {
        message: String,
        descriptor: Descriptor<String>,
        psbt: PartiallySignedTransaction,
    },
}

impl CompletedProposal {
    pub fn spending(tx: Transaction) -> Self {
        Self::Spending { tx }
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

    /// Export Proof of Reserve
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
