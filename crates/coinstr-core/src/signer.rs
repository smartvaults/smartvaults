// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;

use bdk::bitcoin::Network;
use bdk::miniscript::descriptor::{DescriptorKeyParseError, DescriptorType};
use bdk::miniscript::{Descriptor, DescriptorPublicKey};
use hwi::types::HWIDevice;
use hwi::HWIClient;
use keechain_core::bips::bip32::{self, Bip32, Fingerprint};
use keechain_core::types::descriptors::ToDescriptor;
use keechain_core::types::{descriptors, Purpose, Seed};
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::util::{Encryption, Serde};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BIP32(#[from] bip32::Error),
    #[error(transparent)]
    Descriptor(#[from] descriptors::Error),
    #[error(transparent)]
    DescriptorKeyParse(#[from] DescriptorKeyParseError),
    #[error(transparent)]
    HWI(#[from] hwi::error::Error),
    #[error("must be a taproot descriptor")]
    NotTaprootDescriptor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

impl Serde for Signer {}
impl Encryption for Signer {}

impl Signer {
    pub fn new<S>(
        name: S,
        description: Option<S>,
        fingerprint: Fingerprint,
        descriptor: Descriptor<DescriptorPublicKey>,
        t: SignerType,
    ) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        if let DescriptorType::Tr = descriptor.desc_type() {
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
        let descriptor = seed.to_typed_descriptor(Purpose::TR, account, false, network)?;
        Self::new(
            name,
            description,
            seed.fingerprint(network)?,
            descriptor,
            SignerType::Seed,
        )
    }

    pub fn from_hwi<S>(
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
}
