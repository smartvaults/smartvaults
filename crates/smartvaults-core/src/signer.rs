// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;

use bdk::descriptor::IntoWalletDescriptor;
use keechain_core::bips::bip32::{self, Bip32, Fingerprint};
use keechain_core::bips::bip48::ScriptType;
use keechain_core::bitcoin::Network;
use keechain_core::descriptors::{self, ToDescriptor};
use keechain_core::miniscript::descriptor::DescriptorKeyParseError;
use keechain_core::miniscript::DescriptorPublicKey;
use keechain_core::{ColdcardGenericJson, Purpose, Seed};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::SECP256K1;

const PURPOSES: [Purpose; 3] = [
    Purpose::BIP86,
    Purpose::BIP48 {
        script: ScriptType::P2WSH,
    },
    Purpose::BIP48 {
        script: ScriptType::P2TR,
    },
];

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BIP32(#[from] bip32::Error),
    #[error(transparent)]
    Descriptor(#[from] descriptors::Error),
    #[error(transparent)]
    Miniscript(#[from] bdk::miniscript::Error),
    #[error(transparent)]
    DescriptorKeyParse(#[from] DescriptorKeyParseError),
    #[error(transparent)]
    BdkDescriptor(#[from] bdk::descriptor::DescriptorError),
    #[error(transparent)]
    Coldcard(#[from] keechain_core::export::coldcard::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct CoreSigner {
    fingerprint: Fingerprint,
    descriptors: BTreeMap<Purpose, DescriptorPublicKey>,
    // TODO: keep type?
}

impl CoreSigner {
    pub fn new(
        fingerprint: Fingerprint,
        descriptors: BTreeMap<Purpose, DescriptorPublicKey>,
        network: Network,
    ) -> Result<Self, Error> {
        // Check network
        for descriptor in descriptors.values() {
            // TODO: remove this
            descriptor
                .to_string()
                .into_wallet_descriptor(&SECP256K1, network)?;

            // TODO: check if network match
            // TODO: check if fingerprint it's the same for every descriptor
        }

        // Compose signer
        Ok(Self {
            fingerprint,
            descriptors,
        })
    }

    /// Compose [`Signer`] from [`Seed`]
    pub fn from_seed(seed: Seed, account: Option<u32>, network: Network) -> Result<Self, Error> {
        let mut descriptors: BTreeMap<Purpose, DescriptorPublicKey> = BTreeMap::new();

        // Derive descriptors
        for purpose in PURPOSES.into_iter() {
            let descriptor = seed.to_descriptor(purpose, account, false, network, &SECP256K1)?;
            descriptors.insert(purpose, descriptor);
        }

        Self::new(seed.fingerprint(network, &SECP256K1)?, descriptors, network)
    }

    /// Compose [`Signer`] from Coldcard generic JSON (`coldcard-export.json`)
    pub fn from_coldcard(coldcard: ColdcardGenericJson, network: Network) -> Result<Self, Error> {
        let mut descriptors: BTreeMap<Purpose, DescriptorPublicKey> = BTreeMap::new();

        // Derive descriptors
        for purpose in PURPOSES.into_iter() {
            let descriptor = coldcard.descriptor(purpose)?;
            descriptors.insert(purpose, descriptor);
        }

        Self::new(coldcard.fingerprint(), descriptors, network)
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.fingerprint
    }

    pub fn descriptors(&self) -> &BTreeMap<Purpose, DescriptorPublicKey> {
        &self.descriptors
    }

    pub fn descriptor(&self, purpose: Purpose) -> Option<DescriptorPublicKey> {
        self.descriptors.get(&purpose).cloned()
    }
}
