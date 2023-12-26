// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::fmt;

use serde::{Deserialize, Serialize};
use smartvaults_core::bdk::Wallet;
use smartvaults_core::bips::bip32::{self, Bip32, Fingerprint};
use smartvaults_core::bitcoin::Network;
use smartvaults_core::crypto::hash;
use smartvaults_core::descriptors::{self, ToDescriptor};
use smartvaults_core::miniscript::descriptor::{DescriptorKeyParseError, DescriptorType};
use smartvaults_core::miniscript::{Descriptor, DescriptorPublicKey};
use smartvaults_core::{Purpose, Seed, SECP256K1};
use thiserror::Error;

use crate::v1::constants::SMARTVAULTS_ACCOUNT_INDEX;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BIP32(#[from] bip32::Error),
    #[error(transparent)]
    Descriptor(#[from] descriptors::Error),
    #[error(transparent)]
    Miniscript(#[from] smartvaults_core::bdk::miniscript::Error),
    #[error(transparent)]
    DescriptorKeyParse(#[from] DescriptorKeyParseError),
    #[error(transparent)]
    BdkDescriptor(#[from] smartvaults_core::bdk::descriptor::DescriptorError),
    #[error("must be a taproot descriptor")]
    NotTaprootDescriptor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum SignerType {
    Seed,
    Hardware,
    AirGap,
}

impl fmt::Display for SignerType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SignerType::Seed => write!(f, "Seed"),
            SignerType::Hardware => write!(f, "Hardware"),
            SignerType::AirGap => write!(f, "AirGap"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Signer {
    name: String,
    description: Option<String>,
    fingerprint: Fingerprint,
    descriptor: Descriptor<DescriptorPublicKey>,
    t: SignerType,
}

impl fmt::Display for Signer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.t, self.fingerprint)
    }
}

impl Signer {
    fn new<S>(
        name: S,
        description: Option<S>,
        fingerprint: Fingerprint,
        descriptor: Descriptor<DescriptorPublicKey>,
        t: SignerType,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        if let DescriptorType::Tr = descriptor.desc_type() {
            // Check network
            Wallet::new_no_persist(&descriptor.to_string(), None, network)?;

            // Compose signer
            Ok(Self {
                name: name.into(),
                description: description.map(|d| d.into()),
                fingerprint,
                descriptor,
                t,
            })
        } else {
            Err(Error::NotTaprootDescriptor)
        }
    }

    pub fn from_seed<S>(
        name: S,
        description: Option<S>,
        seed: Seed,
        account: Option<u32>,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let descriptor =
            seed.to_typed_descriptor(Purpose::BIP86, account, false, network, &SECP256K1)?;
        Self::new(
            name,
            description,
            seed.fingerprint(network, &SECP256K1)?,
            descriptor,
            SignerType::Seed,
            network,
        )
    }

    pub fn airgap<S>(
        name: S,
        description: Option<S>,
        fingerprint: Fingerprint,
        descriptor: Descriptor<DescriptorPublicKey>,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        Self::new(
            name,
            description,
            fingerprint,
            descriptor,
            SignerType::AirGap,
            network,
        )
    }

    pub fn name(&self) -> String {
        self.name.clone()
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.fingerprint
    }

    pub fn descriptor(&self) -> Descriptor<DescriptorPublicKey> {
        self.descriptor.clone()
    }

    pub fn descriptor_public_key(&self) -> Result<DescriptorPublicKey, Error> {
        match &self.descriptor {
            Descriptor::Tr(tr) => Ok(tr.internal_key().clone()),
            _ => Err(Error::NotTaprootDescriptor),
        }
    }

    pub fn signer_type(&self) -> SignerType {
        self.t
    }

    /// Generate deterministic identifier
    pub fn generate_identifier(&self, network: Network) -> String {
        let unhashed: String = format!("{}:{}", network.magic(), self.fingerprint);
        let hash: String = hash::sha256(unhashed.as_bytes()).to_string();
        hash[..32].to_string()
    }

    pub fn to_shared_signer(&self) -> SharedSigner {
        SharedSigner::from(self.clone())
    }
}

pub fn smartvaults_signer(seed: Seed, network: Network) -> Result<Signer, Error> {
    Signer::from_seed(
        "SmartVaults",
        None,
        seed,
        Some(SMARTVAULTS_ACCOUNT_INDEX),
        network,
    )
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SharedSigner {
    fingerprint: Fingerprint,
    descriptor: Descriptor<DescriptorPublicKey>,
}

impl From<Signer> for SharedSigner {
    fn from(value: Signer) -> Self {
        Self {
            fingerprint: value.fingerprint,
            descriptor: value.descriptor,
        }
    }
}

impl SharedSigner {
    pub fn fingerprint(&self) -> Fingerprint {
        self.fingerprint
    }

    pub fn descriptor(&self) -> Descriptor<DescriptorPublicKey> {
        self.descriptor.clone()
    }

    pub fn descriptor_public_key(&self) -> Result<DescriptorPublicKey, Error> {
        match &self.descriptor {
            Descriptor::Tr(tr) => Ok(tr.internal_key().clone()),
            _ => Err(Error::NotTaprootDescriptor),
        }
    }
}
