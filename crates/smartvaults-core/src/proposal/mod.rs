// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::cmp::Ordering;

use keechain_core::bdk::signer::{SignerError, SignerWrapper};
use keechain_core::bdk::{SignOptions, Wallet};
use keechain_core::bitcoin::psbt::{
    Error as PsbtError, PartiallySignedTransaction, PsbtParseError,
};
use keechain_core::bitcoin::{Network, PrivateKey};
use keechain_core::miniscript::psbt::PsbtExt;
use keechain_core::miniscript::Descriptor;
use keechain_core::psbt::{Error as KPsbtError, PsbtUtility};
use keechain_core::types::Seed;

mod completed;

pub use self::completed::CompletedProposal;
use crate::SECP256K1;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    BdkSigner(#[from] SignerError),
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
    #[error("impossible to finalize the PSBT: {0:?}")]
    ImpossibleToFinalizePsbt(Vec<keechain_core::miniscript::psbt::Error>),
    #[error("impossible to finalize the non-std PSBT")]
    ImpossibleToFinalizeNonStdPsbt,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Proposal {
    Spending {
        descriptor: Descriptor<String>,
        amount: u64,
        psbt: PartiallySignedTransaction,
    },
    ProofOfReserve {
        descriptor: Descriptor<String>,
        message: String,
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
    pub fn spending(
        descriptor: Descriptor<String>,
        amount: u64,
        psbt: PartiallySignedTransaction,
    ) -> Self {
        Self::Spending {
            descriptor,
            amount,
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

    pub fn descriptor(&self) -> Descriptor<String> {
        match self {
            Self::Spending { descriptor, .. } => descriptor.clone(),
            Self::ProofOfReserve { descriptor, .. } => descriptor.clone(),
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
    ) -> Result<PartiallySignedTransaction, Error> {
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

    // pub fn approve_with_hwi_signer(
    // &self,
    // signer: Signer,
    // network: Network,
    // ) -> Result<ApprovedProposal, Error> {
    // let client = HWIClient::find_device(
    // None,
    // None,
    // Some(&signer.fingerprint().to_string()),
    // false,
    // network,
    // )?;
    // let base_psbt = self.psbt();
    // let hwi_psbt = client.sign_tx(&base_psbt)?;
    // if hwi_psbt.psbt != base_psbt {
    // match self {
    // Proposal::Spending { .. } => Ok(ApprovedProposal::spending(hwi_psbt.psbt)),
    // Proposal::ProofOfReserve { .. } => {
    // Ok(ApprovedProposal::proof_of_reserve(hwi_psbt.psbt))
    // }
    // }
    // } else {
    // Err(Error::PsbtNotSigned)
    // }
    // }

    pub fn finalize<I>(&self, psbts: I, network: Network) -> Result<CompletedProposal, Error>
    where
        I: IntoIterator<Item = PartiallySignedTransaction>,
    {
        let mut base_psbt: PartiallySignedTransaction = self.psbt();

        // Combine PSBTs
        for psbt in psbts.into_iter() {
            base_psbt.combine(psbt)?;
        }

        // Finalize the proposal
        match self {
            Self::Spending { .. } => {
                base_psbt
                    .finalize_mut(&SECP256K1)
                    .map_err(Error::ImpossibleToFinalizePsbt)?;
                Ok(CompletedProposal::spending(base_psbt.extract_tx()))
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
