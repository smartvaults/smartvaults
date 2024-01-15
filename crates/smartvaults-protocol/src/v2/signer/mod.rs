// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Signer

use core::fmt;
use core::hash::{Hash, Hasher};
use core::ops::Deref;
use std::collections::BTreeMap;

use nostr::{Event, EventBuilder, Keys, Tag};
use prost::Message;
use smartvaults_core::bips::bip32::Fingerprint;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::DescriptorPublicKey;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::{ColdcardGenericJson, CoreSigner, Purpose, Seed};

pub mod id;
mod proto;
pub mod shared;

pub use self::id::SignerIdentifier;
pub use self::shared::SharedSigner;
use super::constants::{SIGNER_KIND_V2, SMARTVAULTS_ACCOUNT_INDEX};
use super::message::{EncodingVersion, ProtocolEncoding, ProtocolEncryption};
use super::NostrPublicIdentifier;
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
#[derive(Debug, Clone, PartialOrd, Ord)]
pub struct Signer {
    name: String,
    description: String,
    core: CoreSigner,
    r#type: SignerType,
}

impl PartialEq for Signer {
    fn eq(&self, other: &Self) -> bool {
        self.core == other.core
    }
}

impl Eq for Signer {}

impl Hash for Signer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.core.hash(state)
    }
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
    pub fn from_seed(seed: &Seed, account: Option<u32>, network: Network) -> Result<Self, Error> {
        let core: CoreSigner = CoreSigner::from_seed(seed, account, network)?;
        Ok(Self::new(core, SignerType::Seed))
    }

    /// Compose Smart Vaults signer (custom account index)
    pub fn smartvaults(seed: &Seed, network: Network) -> Result<Self, Error> {
        Self::from_seed(seed, Some(SMARTVAULTS_ACCOUNT_INDEX), network)
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

    /// Generate unique deterministic identifier
    ///
    /// WARNING: the deterministic identifier it's generated using the `fingerprint`
    /// so if it change, the deterministic identifer will be different.
    pub fn id(&self) -> SignerIdentifier {
        SignerIdentifier::from((self.network(), self.fingerprint()))
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

    /// Generate deterministic Nostr Public Identifier
    pub fn nostr_public_identifier(&self) -> NostrPublicIdentifier {
        NostrPublicIdentifier::from(*self.id())
    }

    /// Get Shared Signer
    pub fn as_shared(&self, owner: XOnlyPublicKey, receiver: XOnlyPublicKey) -> SharedSigner {
        SharedSigner::new(owner, receiver, self.core.clone())
    }

    /// Consume [`Signer`] and get Shared Signer
    pub fn to_shared(self, owner: XOnlyPublicKey, receiver: XOnlyPublicKey) -> SharedSigner {
        SharedSigner::new(owner, receiver, self.core)
    }
}

impl ProtocolEncoding for Signer {
    type Err = Error;

    fn protocol_network(&self) -> Network {
        self.network()
    }

    fn pre_encoding(&self) -> (EncodingVersion, Vec<u8>) {
        let proposal: ProtoSigner = self.into();
        (EncodingVersion::ProtoBuf, proposal.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoSigner = ProtoSigner::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for Signer {
    type Err = Error;
}

/// Build [`Signer`] event
///
/// Must use **own** [`Keys`] (not random or shared key)!
pub fn build_event(keys: &Keys, signer: &Signer) -> Result<Event, Error> {
    // Encrypt
    let encrypted_content: String = signer.encrypt_with_keys(keys)?;

    // Compose and build event
    let identifier: String = signer.nostr_public_identifier().to_string();
    Ok(EventBuilder::new(
        SIGNER_KIND_V2,
        encrypted_content,
        [Tag::Identifier(identifier)],
    )
    .to_event(keys)?)
}
