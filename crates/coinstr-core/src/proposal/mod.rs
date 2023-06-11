// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;

use bdk::bitcoin::{Network, PrivateKey};
use bdk::database::MemoryDatabase;
use bdk::miniscript::psbt::PsbtExt;
use bdk::miniscript::Descriptor;
use bdk::signer::SignerWrapper;
use bdk::{SignOptions, Wallet};
#[cfg(feature = "hwi")]
use hwi::HWIClient;
use keechain_core::bitcoin::psbt::{
    Error as PsbtError, PartiallySignedTransaction, PsbtParseError,
};
use keechain_core::bitcoin::Address;
use keechain_core::types::psbt::{Error as KPsbtError, Psbt};
use keechain_core::types::Seed;
use keechain_core::SECP256K1;
use serde::{Deserialize, Serialize};

mod approved;
mod completed;

pub use self::approved::ApprovedProposal;
pub use self::completed::CompletedProposal;
#[cfg(feature = "hwi")]
use crate::signer::Signer;
use crate::util::serde::{deserialize_psbt, serialize_psbt};
use crate::util::{Encryption, Serde};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error(transparent)]
    Psbt(#[from] PsbtError),
    #[error(transparent)]
    KPsbt(#[from] KPsbtError),
    #[error(transparent)]
    PsbtParse(#[from] PsbtParseError),
    #[cfg(feature = "hwi")]
    #[error(transparent)]
    HWI(#[from] hwi::error::Error),
    #[error("PSBT not signed (equal to base PSBT)")]
    PsbtNotSigned,
    #[error("approved proposals not proveded")]
    EmptyApprovedProposals,
    #[error("the provided approved proposals must have the same type")]
    ApprovedProposalTypeMismatch,
    #[error("impossible to finalize the PSBT: {0:?}")]
    ImpossibleToFinalizePsbt(Vec<bdk::miniscript::psbt::Error>),
    #[error("impossible to finalize the non-std PSBT")]
    ImpossibleToFinalizeNonStdPsbt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProposalType {
    Spending,
    ProofOfReserve,
}

impl fmt::Display for ProposalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spending => write!(f, "spending"),
            Self::ProofOfReserve => write!(f, "proof-of-reserve"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Proposal {
    Spending {
        descriptor: Descriptor<String>,
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
        descriptor: Descriptor<String>,
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
        descriptor: Descriptor<String>,
        to_address: Address,
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

    pub fn get_type(&self) -> ProposalType {
        match self {
            Self::Spending { .. } => ProposalType::Spending,
            Self::ProofOfReserve { .. } => ProposalType::ProofOfReserve,
        }
    }

    pub fn descriptor(&self) -> Descriptor<String> {
        match self {
            Self::Spending { descriptor, .. } => descriptor.clone(),
            Self::ProofOfReserve { descriptor, .. } => descriptor.clone(),
        }
    }

    pub fn description(&self) -> String {
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

    pub fn approve(
        &self,
        seed: &Seed,
        custom_signers: Vec<SignerWrapper<PrivateKey>>,
        network: Network,
    ) -> Result<ApprovedProposal, Error> {
        // Sign the transaction
        let mut psbt: PartiallySignedTransaction = self.psbt();
        let _finalized: bool =
            psbt.sign_custom(seed, Some(self.descriptor()), custom_signers, network)?;

        match self {
            Proposal::Spending { .. } => Ok(ApprovedProposal::spending(psbt)),
            Proposal::ProofOfReserve { .. } => Ok(ApprovedProposal::proof_of_reserve(psbt)),
        }
    }

    pub fn approve_with_signed_psbt(
        &self,
        signed_psbt: PartiallySignedTransaction,
    ) -> Result<ApprovedProposal, Error> {
        if signed_psbt != self.psbt() {
            // TODO: check if psbt was signed with the correct signer
            match self {
                Proposal::Spending { .. } => Ok(ApprovedProposal::spending(signed_psbt)),
                Proposal::ProofOfReserve { .. } => {
                    Ok(ApprovedProposal::proof_of_reserve(signed_psbt))
                }
            }
        } else {
            Err(Error::PsbtNotSigned)
        }
    }

    #[cfg(feature = "hwi")]
    pub fn approve_with_hwi_signer(
        &self,
        signer: Signer,
        network: Network,
    ) -> Result<ApprovedProposal, Error> {
        let client = HWIClient::find_device(
            None,
            None,
            Some(&signer.fingerprint().to_string()),
            false,
            network,
        )?;
        let base_psbt = self.psbt();
        let hwi_psbt = client.sign_tx(&base_psbt)?;
        if hwi_psbt.psbt != base_psbt {
            match self {
                Proposal::Spending { .. } => Ok(ApprovedProposal::spending(hwi_psbt.psbt)),
                Proposal::ProofOfReserve { .. } => {
                    Ok(ApprovedProposal::proof_of_reserve(hwi_psbt.psbt))
                }
            }
        } else {
            Err(Error::PsbtNotSigned)
        }
    }

    pub fn finalize(
        &self,
        approved_proposals: Vec<ApprovedProposal>,
        network: Network,
    ) -> Result<CompletedProposal, Error> {
        let mut base_psbt: PartiallySignedTransaction = self.psbt();

        let first_type: ProposalType = approved_proposals
            .first()
            .ok_or(Error::EmptyApprovedProposals)?
            .get_type();

        // Combine PSBTs
        for proposal in approved_proposals.into_iter() {
            if proposal.get_type() != first_type {
                return Err(Error::ApprovedProposalTypeMismatch);
            }
            base_psbt.combine(proposal.psbt())?;
        }

        // Finalize the proposal
        match first_type {
            ProposalType::Spending => {
                base_psbt
                    .finalize_mut(&SECP256K1)
                    .map_err(Error::ImpossibleToFinalizePsbt)?;
                Ok(CompletedProposal::spending(
                    base_psbt.extract_tx(),
                    self.description(),
                ))
            }
            ProposalType::ProofOfReserve => {
                let db = MemoryDatabase::new();
                let wallet = Wallet::new(&self.descriptor().to_string(), None, network, db)?;
                let signopts = SignOptions {
                    trust_witness_utxo: true,
                    remove_partial_sigs: false,
                    ..Default::default()
                };
                if wallet.finalize_psbt(&mut base_psbt, signopts)? {
                    Ok(CompletedProposal::proof_of_reserve(
                        self.description(),
                        self.descriptor(),
                        base_psbt,
                    ))
                } else {
                    Err(Error::ImpossibleToFinalizeNonStdPsbt)
                }
            }
        }
    }
}

impl Serde for Proposal {}
impl Encryption for Proposal {}
