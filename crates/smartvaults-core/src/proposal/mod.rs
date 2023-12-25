// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;
use std::fmt;

use keechain_core::bdk::signer::SignerWrapper;
use keechain_core::bdk::{SignOptions, Wallet};
use keechain_core::bitcoin::address::NetworkUnchecked;
use keechain_core::bitcoin::psbt::{
    Error as PsbtError, PartiallySignedTransaction, PsbtParseError,
};
use keechain_core::bitcoin::Address;
use keechain_core::bitcoin::{Network, PrivateKey};
use keechain_core::miniscript::psbt::PsbtExt;
use keechain_core::miniscript::Descriptor;
use keechain_core::psbt::{Error as KPsbtError, PsbtUtility};
use keechain_core::types::Seed;
use serde::{Deserialize, Serialize};

mod approved;
mod completed;

pub use self::approved::ApprovedProposal;
pub use self::completed::CompletedProposal;
use crate::util::{deserialize_psbt, serialize_psbt};
use crate::SECP256K1;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bdk(#[from] keechain_core::bdk::Error),
    #[error(transparent)]
    BdkDescriptor(#[from] keechain_core::bdk::descriptor::DescriptorError),
    #[error(transparent)]
    Psbt(#[from] PsbtError),
    #[error(transparent)]
    KPsbt(#[from] KPsbtError),
    #[error(transparent)]
    PsbtParse(#[from] PsbtParseError),
    #[error("PSBT not signed (equal to base PSBT)")]
    PsbtNotSigned,
    #[error("approved proposals not proveded")]
    EmptyApprovedProposals,
    #[error("the provided approved proposals must have the same type")]
    ApprovedProposalTypeMismatch,
    #[error("impossible to finalize the PSBT: {0:?}")]
    ImpossibleToFinalizePsbt(Vec<keechain_core::miniscript::psbt::Error>),
    #[error("impossible to finalize the non-std PSBT")]
    ImpossibleToFinalizeNonStdPsbt,
}

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

    pub fn approve(
        &self,
        seed: &Seed,
        custom_signers: Vec<SignerWrapper<PrivateKey>>,
        network: Network,
    ) -> Result<ApprovedProposal, Error> {
        let mut psbt: PartiallySignedTransaction = self.psbt();
        psbt.sign_custom(
            seed,
            Some(self.descriptor()),
            custom_signers.clone(),
            network,
            &SECP256K1,
        )?;

        match self {
            Proposal::Spending { .. } => Ok(ApprovedProposal::spending(psbt)),
            Proposal::ProofOfReserve { .. } => Ok(ApprovedProposal::proof_of_reserve(psbt)),
            Proposal::KeyAgentPayment { .. } => Ok(ApprovedProposal::key_agent_payment(psbt)),
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
                Proposal::KeyAgentPayment { .. } => {
                    Ok(ApprovedProposal::key_agent_payment(signed_psbt))
                }
            }
        } else {
            Err(Error::PsbtNotSigned)
        }
    }

    /* pub fn approve_with_hwi_signer(
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
    } */

    pub fn finalize<I>(
        &self,
        approved_proposals: I,
        network: Network,
    ) -> Result<CompletedProposal, Error>
    where
        I: IntoIterator<Item = ApprovedProposal>,
    {
        let mut base_psbt: PartiallySignedTransaction = self.psbt();

        // Combine PSBTs
        for proposal in approved_proposals.into_iter() {
            if proposal.get_type() != self.get_type() {
                return Err(Error::ApprovedProposalTypeMismatch);
            }
            base_psbt.combine(proposal.psbt())?;
        }

        // Finalize the proposal
        match self {
            Self::Spending { description, .. } => {
                base_psbt
                    .finalize_mut(&SECP256K1)
                    .map_err(Error::ImpossibleToFinalizePsbt)?;
                Ok(CompletedProposal::spending(
                    base_psbt.extract_tx(),
                    description,
                ))
            }
            Self::KeyAgentPayment {
                signer_descriptor,
                description,
                period,
                ..
            } => {
                base_psbt
                    .finalize_mut(&SECP256K1)
                    .map_err(Error::ImpossibleToFinalizePsbt)?;
                Ok(CompletedProposal::key_agent_payment(
                    base_psbt.extract_tx(),
                    signer_descriptor.clone(),
                    description,
                    *period,
                ))
            }
            Self::ProofOfReserve {
                descriptor,
                message,
                ..
            } => {
                let wallet = Wallet::new_no_persist(&descriptor.to_string(), None, network)?;
                let signopts = SignOptions {
                    trust_witness_utxo: true,
                    remove_partial_sigs: false,
                    ..Default::default()
                };
                if wallet.finalize_psbt(&mut base_psbt, signopts)? {
                    Ok(CompletedProposal::proof_of_reserve(
                        message,
                        descriptor.clone(),
                        base_psbt,
                    ))
                } else {
                    Err(Error::ImpossibleToFinalizeNonStdPsbt)
                }
            }
        }
    }
}
