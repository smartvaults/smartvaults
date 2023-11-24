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

mod reserves;
mod spending;

pub use self::reserves::ProofOfReserveProposal;
pub use self::spending::SpendingProposal;
use crate::SECP256K1;
#[cfg(feature = "hwi")]
use crate::{hwi, CoreSigner};

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
    #[cfg(feature = "hwi")]
    #[error(transparent)]
    HWI(#[from] hwi::Error),
    #[error("PSBT not signed (equal to base PSBT)")]
    PsbtNotSigned,
    #[error("approved proposals not proveded")]
    EmptyApprovedProposals,
    #[error("impossible to finalize the PSBT: {0:?}")]
    ImpossibleToFinalizePsbt(Vec<keechain_core::miniscript::psbt::Error>),
    #[error("impossible to finalize the non-std PSBT")]
    ImpossibleToFinalizeNonStdPsbt,
}

#[cfg_attr(feature = "hwi", async_trait::async_trait)]
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
        Ok(psbt)
    }

    #[cfg(feature = "hwi")]
    async fn approve_with_hwi(
        &self,
        signer: CoreSigner,
    ) -> Result<PartiallySignedTransaction, Error> {
        let mut base_psbt = self.psbt();
        let device = hwi::find_device(signer.fingerprint(), self.network()).await?;
        device
            .sign_tx(&mut base_psbt)
            .await
            .map_err(|e| Error::HWI(hwi::Error::HWI(e)))?;
        Ok(base_psbt)
    }

    fn finalize<I>(&self, psbts: I) -> Result<T, Error>
    where
        I: IntoIterator<Item = PartiallySignedTransaction>;
}
