// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use keechain_core::bdk::signer::{SignerError, SignerWrapper};
use keechain_core::bitcoin::psbt::{
    Error as PsbtError, PartiallySignedTransaction, PsbtParseError,
};
use keechain_core::bitcoin::{Network, PrivateKey};
use keechain_core::miniscript::Descriptor;
use keechain_core::psbt::{Error as KPsbtError, PsbtUtility};
use keechain_core::types::Seed;

#[cfg(feature = "reserves")]
mod reserves;
mod spending;

#[cfg(feature = "reserves")]
pub use self::reserves::ProofOfReserveProposal;
pub use self::spending::SpendingProposal;
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

pub trait ProposalSigning<T> {
    fn psbt(&self) -> PartiallySignedTransaction;

    fn descriptor(&self) -> Descriptor<String>;

    fn network(&self) -> Network;

    fn approve(
        &self,
        seed: &Seed,
        custom_signers: Vec<SignerWrapper<PrivateKey>>,
    ) -> Result<PartiallySignedTransaction, Error> {
        let mut psbt: PartiallySignedTransaction = self.psbt();
        psbt.sign_custom(
            seed,
            Some(self.descriptor()),
            custom_signers.clone(),
            self.network(),
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

    fn finalize<I>(&self, psbts: I) -> Result<T, Error>
    where
        I: IntoIterator<Item = PartiallySignedTransaction>;
}
