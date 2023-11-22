// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Signer

use core::fmt;
use core::ops::Deref;
use std::collections::BTreeMap;

use prost::Message;
use smartvaults_core::bips::bip32::Fingerprint;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::DescriptorPublicKey;
use smartvaults_core::{ColdcardGenericJson, CoreSigner, Purpose, Seed};

mod proto;

use super::core::{ProtocolEncoding, ProtocolEncryption, SchemaVersion};
use crate::v2::proto::signer::ProtoSigner;
use crate::v2::Error;

/// Signer Type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SignerType {
    /// Seed
    Seed,
    /// Signing Device (aka Hardware Wallet) that can be used
    /// with USB, Bluetooth or other that provides a direct connection with the wallet.
    Hardware,
    /// Signing Device that can be used without ever being connected
    /// to online devices, via microSD or camera.
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

/// Signer
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Signer {
    name: String,
    description: String,
    core: CoreSigner,
    r#type: SignerType,
}

impl Deref for Signer {
    type Target = CoreSigner;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

impl Signer {
    fn new(core: CoreSigner, r#type: SignerType) -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            core,
            r#type,
        }
    }

    /// Compose [`Signer`] from [`Seed`]
    pub fn from_seed(seed: Seed, account: Option<u32>, network: Network) -> Result<Self, Error> {
        let core: CoreSigner = CoreSigner::from_seed(seed, account, network)?;
        Ok(Self::new(core, SignerType::Seed))
    }

    /// Compose [`Signer`] from custom airgap device
    pub fn airgap(
        fingerprint: Fingerprint,
        descriptors: BTreeMap<Purpose, DescriptorPublicKey>,
        network: Network,
    ) -> Result<Self, Error> {
        let core: CoreSigner = CoreSigner::new(fingerprint, descriptors, network)?;
        Ok(Self::new(core, SignerType::AirGap))
    }

    /// Compose [`Signer`] from Coldcard generic JSON (`coldcard-export.json`)
    pub fn from_coldcard(coldcard: ColdcardGenericJson, network: Network) -> Result<Self, Error> {
        let core: CoreSigner = CoreSigner::from_coldcard(coldcard, network)?;
        Ok(Self::new(core, SignerType::AirGap))
    }

    /// Get [`Signer`] name
    pub fn name(&self) -> String {
        self.name.clone()
    }

    /// Get [`Signer`] description
    pub fn description(&self) -> String {
        self.description.clone()
    }

    /// Get [`Signer`] type
    pub fn r#type(&self) -> SignerType {
        self.r#type
    }

    /// Change signer name
    pub fn change_name<S>(&mut self, name: S)
    where
        S: Into<String>,
    {
        self.name = name.into();
    }

    /// Change signer description
    pub fn change_description<S>(&mut self, description: S)
    where
        S: Into<String>,
    {
        self.description = description.into();
    }
}

impl ProtocolEncoding for Signer {
    type Err = Error;

    fn pre_encoding(&self) -> (SchemaVersion, Vec<u8>) {
        let proposal: ProtoSigner = self.into();
        (SchemaVersion::ProtoBuf, proposal.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoSigner = ProtoSigner::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for Signer {
    type Err = Error;
}
