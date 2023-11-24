// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::cmp::Ordering;
use core::fmt;

use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::Address;
use smartvaults_core::miniscript::Descriptor;

mod approved;
mod completed;

pub use self::approved::ApprovedProposal;
pub use self::completed::CompletedProposal;
use crate::v1::psbt::{deserialize_psbt, serialize_psbt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProposalType {
    Spending,
    ProofOfReserve,
    KeyAgentPayment,
}

impl fmt::Display for ProposalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spending => write!(f, "spending"),
            Self::ProofOfReserve => write!(f, "proof-of-reserve"),
            Self::KeyAgentPayment => write!(f, "key-agent-payment"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Period {
    /// From timestamp
    pub from: u64,
    /// To timestamp
    pub to: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Proposal {
    Spending {
        descriptor: Descriptor<String>,
        to_address: Address<NetworkUnchecked>,
        amount: u64,
        description: String,
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
    ProofOfReserve {
        descriptor: Descriptor<String>,
        message: String,
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
    KeyAgentPayment {
        descriptor: Descriptor<String>,
        /// Needed to indentify the Key Agent and the signer used
        signer_descriptor: Descriptor<String>,
        amount: u64,
        description: String,
        period: Period,
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
}

impl PartialOrd for Proposal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Proposal {
    fn cmp(&self, other: &Self) -> Ordering {
        other.psbt().unsigned_tx.cmp(&self.psbt().unsigned_tx)
    }
}

impl Proposal {
    pub fn spending<S>(
        descriptor: Descriptor<String>,
        to_address: Address<NetworkUnchecked>,
        amount: u64,
        description: S,
        psbt: PartiallySignedTransaction,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::Spending {
            descriptor,
            to_address,
            amount,
            description: description.into(),
            psbt,
        }
    }

    pub fn proof_of_reserve<S>(
        descriptor: Descriptor<String>,
        message: S,
        psbt: PartiallySignedTransaction,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::ProofOfReserve {
            descriptor,
            message: message.into(),
            psbt,
        }
    }

    pub fn key_agent_payment<S>(
        descriptor: Descriptor<String>,
        signer_descriptor: Descriptor<String>,
        amount: u64,
        description: S,
        period: Period,
        psbt: PartiallySignedTransaction,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::KeyAgentPayment {
            descriptor,
            signer_descriptor,
            amount,
            description: description.into(),
            period,
            psbt,
        }
    }

    pub fn get_type(&self) -> ProposalType {
        match self {
            Self::Spending { .. } => ProposalType::Spending,
            Self::ProofOfReserve { .. } => ProposalType::ProofOfReserve,
            Self::KeyAgentPayment { .. } => ProposalType::KeyAgentPayment,
        }
    }

    pub fn descriptor(&self) -> Descriptor<String> {
        match self {
            Self::Spending { descriptor, .. } => descriptor.clone(),
            Self::ProofOfReserve { descriptor, .. } => descriptor.clone(),
            Self::KeyAgentPayment { descriptor, .. } => descriptor.clone(),
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::Spending { description, .. } => description.clone(),
            Self::ProofOfReserve { message, .. } => message.clone(),
            Self::KeyAgentPayment { description, .. } => description.clone(),
        }
    }

    pub fn psbt(&self) -> PartiallySignedTransaction {
        match self {
            Self::Spending { psbt, .. } => psbt.clone(),
            Self::ProofOfReserve { psbt, .. } => psbt.clone(),
            Self::KeyAgentPayment { psbt, .. } => psbt.clone(),
        }
    }
}
