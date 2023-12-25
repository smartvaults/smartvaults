// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::fmt;

use bdk::miniscript::descriptor::Tr;
use bdk::Wallet;
use keechain_core::bips::bip32::{self, Bip32, Fingerprint};
use keechain_core::bips::bip48::ScriptType;
use keechain_core::bitcoin::Network;
use keechain_core::crypto::hash;
use keechain_core::descriptors::{self, ToDescriptor};
use keechain_core::miniscript::descriptor::{DescriptorKeyParseError, DescriptorType};
use keechain_core::miniscript::{Descriptor, DescriptorPublicKey};
use keechain_core::{ColdcardGenericJson, Purpose, Seed};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::constants::SMARTVAULTS_ACCOUNT_INDEX;
use crate::SECP256K1;

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

    /* pub fn from_hwi<S>(
        name: S,
        description: Option<S>,
        device: HWIDevice,
        account: Option<u32>,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let client = HWIClient::get_client(&device, false, network)?;
        let path = bip32::account_extended_path(86, network, account)?;
        let xpub = client.get_xpub(&path, false)?;
        let descriptor =
            descriptors::typed_descriptor(device.fingerprint, xpub.xpub, &path, false)?;
        Self::new(
            name,
            description,
            device.fingerprint,
            descriptor,
            SignerType::Hardware,
        )
    } */

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

    /// Build [`Signer`] from Coldcard generic JSON
    pub fn from_coldcard<S>(
        name: S,
        coldcard: ColdcardGenericJson,
        network: Network,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        let descriptor = coldcard.descriptor(Purpose::BIP48 {
            script: ScriptType::P2TR,
        })?;
        let descriptor = Descriptor::Tr(Tr::new(descriptor, None)?);
        Self::airgap(name, None, coldcard.fingerprint(), descriptor, network)
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
