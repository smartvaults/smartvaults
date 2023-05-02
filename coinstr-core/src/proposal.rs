// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use bdk::bitcoin::{Txid, XOnlyPublicKey};
use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::Address;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::Encryption;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Proposal {
    Spending {
        to_address: Address,
        amount: u64,
        description: String,
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
    ProofOfReserve {
        message: String,
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
}

impl Proposal {
    pub fn spending<S>(
        to_address: Address,
        amount: u64,
        description: S,
        psbt: PartiallySignedTransaction,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::Spending {
            to_address,
            amount,
            description: description.into(),
            psbt,
        }
    }

    pub fn proof_of_reserve<S>(message: S, psbt: PartiallySignedTransaction) -> Self
    where
        S: Into<String>,
    {
        Self::ProofOfReserve {
            message: message.into(),
            psbt,
        }
    }

    pub fn desc(&self) -> String {
        match self {
            Self::Spending { description, .. } => description.clone(),
            Self::ProofOfReserve { message, .. } => message.clone(),
        }
    }

    pub fn psbt(&self) -> PartiallySignedTransaction {
        match self {
            Self::Spending { psbt, .. } => psbt.clone(),
            Self::ProofOfReserve { psbt, .. } => psbt.clone(),
        }
    }
}

impl Encryption for Proposal {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedProposal {
    #[serde(
        serialize_with = "serialize_psbt",
        deserialize_with = "deserialize_psbt"
    )]
    pub psbt: PartiallySignedTransaction,
}

impl ApprovedProposal {
    pub fn new(psbt: PartiallySignedTransaction) -> Self {
        Self { psbt }
    }

    pub fn psbt(&self) -> PartiallySignedTransaction {
        self.psbt.clone()
    }
}

impl Encryption for ApprovedProposal {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompletedProposal {
    Spending {
        txid: Txid,
        description: String,
        approvals: Vec<XOnlyPublicKey>,
    },
    ProofOfReserve {
        message: String,
        approvals: Vec<XOnlyPublicKey>,
        #[serde(
            serialize_with = "serialize_psbt",
            deserialize_with = "deserialize_psbt"
        )]
        psbt: PartiallySignedTransaction,
    },
}

impl CompletedProposal {
    pub fn spending<S>(txid: Txid, description: S, approvals: Vec<XOnlyPublicKey>) -> Self
    where
        S: Into<String>,
    {
        Self::Spending {
            txid,
            description: description.into(),
            approvals,
        }
    }

    pub fn proof_of_reserve<S>(
        message: S,
        approvals: Vec<XOnlyPublicKey>,
        psbt: PartiallySignedTransaction,
    ) -> Self
    where
        S: Into<String>,
    {
        Self::ProofOfReserve {
            message: message.into(),
            approvals,
            psbt,
        }
    }

    pub fn txid(&self) -> Option<Txid> {
        match self {
            Self::Spending { txid, .. } => Some(*txid),
            _ => None,
        }
    }
}

impl Encryption for CompletedProposal {}

fn serialize_psbt<S>(psbt: &PartiallySignedTransaction, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&psbt.to_string())
}

fn deserialize_psbt<'de, D>(deserializer: D) -> Result<PartiallySignedTransaction, D::Error>
where
    D: Deserializer<'de>,
{
    let psbt = String::deserialize(deserializer)?;
    PartiallySignedTransaction::from_str(&psbt).map_err(serde::de::Error::custom)
}
