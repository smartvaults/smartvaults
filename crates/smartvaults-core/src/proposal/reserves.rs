// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::cmp::Ordering;

use keechain_core::bdk::{SignOptions, Wallet};
use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::Network;
use keechain_core::miniscript::Descriptor;

use super::{Error, ProposalSigning};
use crate::ProofOfReserve;

/// Proof of Reserve proposal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProofOfReserveProposal {
    pub descriptor: Descriptor<String>,
    pub message: String,
    pub psbt: PartiallySignedTransaction,
    pub network: Network,
}

impl PartialOrd for ProofOfReserveProposal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ProofOfReserveProposal {
    fn cmp(&self, other: &Self) -> Ordering {
        other.psbt.unsigned_tx.cmp(&self.psbt.unsigned_tx)
    }
}

#[cfg_attr(feature = "hwi", async_trait::async_trait)]
impl ProposalSigning<ProofOfReserve> for ProofOfReserveProposal {
    fn psbt(&self) -> PartiallySignedTransaction {
        self.psbt.clone()
    }

    fn descriptor(&self) -> Descriptor<String> {
        self.descriptor.clone()
    }

    fn network(&self) -> Network {
        self.network
    }

    fn finalize<I>(&self, psbts: I) -> Result<ProofOfReserve, Error>
    where
        I: IntoIterator<Item = PartiallySignedTransaction>,
    {
        let mut base_psbt: PartiallySignedTransaction = self.psbt();

        // Combine PSBTs
        for psbt in psbts.into_iter() {
            base_psbt.combine(psbt)?;
        }

        // Finalize the proposal
        let wallet = Wallet::new_no_persist(&self.descriptor.to_string(), None, self.network)?;
        let signopts = SignOptions {
            trust_witness_utxo: true,
            remove_partial_sigs: false,
            ..Default::default()
        };
        if wallet.finalize_psbt(&mut base_psbt, signopts)? {
            Ok(ProofOfReserve {
                descriptor: self.descriptor(),
                message: self.message.clone(),
                psbt: base_psbt,
            })
        } else {
            Err(Error::ImpossibleToFinalizeNonStdPsbt)
        }
    }
}
