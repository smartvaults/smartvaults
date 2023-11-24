// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::cmp::Ordering;

use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::{Network, Transaction};
use keechain_core::miniscript::psbt::PsbtExt;
use keechain_core::miniscript::Descriptor;

use super::{Error, ProposalSigning};
use crate::SECP256K1;

/// Spending proposal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpendingProposal {
    pub descriptor: Descriptor<String>,
    pub psbt: PartiallySignedTransaction,
    pub network: Network,
}

impl PartialOrd for SpendingProposal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpendingProposal {
    fn cmp(&self, other: &Self) -> Ordering {
        other.psbt.unsigned_tx.cmp(&self.psbt.unsigned_tx)
    }
}

impl ProposalSigning<Transaction> for SpendingProposal {
    fn psbt(&self) -> PartiallySignedTransaction {
        self.psbt.clone()
    }

    fn descriptor(&self) -> Descriptor<String> {
        self.descriptor.clone()
    }

    fn network(&self) -> Network {
        self.network
    }

    fn finalize<I>(&self, psbts: I) -> Result<Transaction, Error>
    where
        I: IntoIterator<Item = PartiallySignedTransaction>,
    {
        let mut base_psbt: PartiallySignedTransaction = self.psbt();

        // Combine PSBTs
        for psbt in psbts.into_iter() {
            base_psbt.combine(psbt)?;
        }

        // Finalize the proposal
        base_psbt
            .finalize_mut(&SECP256K1)
            .map_err(Error::ImpossibleToFinalizePsbt)?;
        Ok(base_psbt.extract_tx())
    }
}
