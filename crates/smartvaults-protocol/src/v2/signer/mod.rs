// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Signer

use core::hash::{Hash, Hasher};
use core::ops::Deref;
use std::collections::BTreeMap;

use nostr::{Event, EventBuilder, Keys, Tag, Timestamp};
use prost::Message;
use smartvaults_core::bips::bip32::Fingerprint;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::constants::SMARTVAULTS_ACCOUNT_INDEX;
#[cfg(feature = "hwi")]
use smartvaults_core::hwi::BoxedHWI;
use smartvaults_core::miniscript::DescriptorPublicKey;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::{ColdcardGenericJson, CoreSigner, Purpose, Seed};

pub mod id;
mod proto;
pub mod shared;

pub use self::id::SignerIdentifier;
pub use self::shared::{SharedSigner, SharedSignerInvite};
use super::constants::SIGNER_KIND_V2;
use super::message::{MessageVersion, ProtocolEncoding, ProtocolEncryption};
use super::NostrPublicIdentifier;
use crate::v2::proto::signer::ProtoSigner;
use crate::v2::Error;

/// Signer
#[derive(Debug, Clone, PartialOrd, Ord)]
pub struct Signer {
    name: String,
    description: String,
    core: CoreSigner,
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
    fn new(core: CoreSigner) -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            core,
        }
    }

    /// Compose [`Signer`] from [`Seed`]
    pub fn from_seed(seed: &Seed, account: Option<u32>, network: Network) -> Result<Self, Error> {
        let core: CoreSigner = CoreSigner::from_seed(seed, account, network)?;
        Ok(Self::new(core))
    }

    /// Compose Smart Vaults signer (custom account index)
    pub fn smartvaults(seed: &Seed, network: Network) -> Result<Self, Error> {
        let mut signer = Self::from_seed(seed, Some(SMARTVAULTS_ACCOUNT_INDEX), network)?;
        signer.change_name("SmartVaults");
        signer.change_description("Default SmartVaults signer");
        Ok(signer)
    }

    /// Compose [`Signer`] from custom airgap device
    pub fn airgap(
        fingerprint: Fingerprint,
        descriptors: BTreeMap<Purpose, DescriptorPublicKey>,
        network: Network,
    ) -> Result<Self, Error> {
        let core: CoreSigner = CoreSigner::airgap(fingerprint, descriptors, network)?;
        Ok(Self::new(core))
    }

    /// Compose [`Signer`] from Coldcard generic JSON (`coldcard-export.json`)
    pub fn from_coldcard(coldcard: &ColdcardGenericJson, network: Network) -> Result<Self, Error> {
        let core: CoreSigner = CoreSigner::from_coldcard(coldcard, network)?;
        Ok(Self::new(core))
    }

    /// Compose [Signer] from USB `Hardware Wallet`
    #[cfg(feature = "hwi")]
    pub async fn from_hwi(device: BoxedHWI, network: Network) -> Result<Self, Error> {
        let core: CoreSigner = CoreSigner::from_hwi(device, network).await?;
        Ok(Self::new(core))
    }

    /// Compute unique deterministic identifier
    ///
    /// WARNING: the deterministic identifier it's generated using the `fingerprint`
    /// so if it change, the deterministic identifer will be different!
    pub fn compute_id(&self) -> SignerIdentifier {
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
        NostrPublicIdentifier::from(*self.compute_id())
    }

    /// Get Shared Signer
    pub fn as_shared(&self, owner: XOnlyPublicKey, receiver: XOnlyPublicKey) -> SharedSigner {
        SharedSigner::new(owner, receiver, self.core.clone(), Timestamp::now())
    }

    /// Consume [`Signer`] and get Shared Signer
    pub fn to_shared(self, owner: XOnlyPublicKey, receiver: XOnlyPublicKey) -> SharedSigner {
        SharedSigner::new(owner, receiver, self.core, Timestamp::now())
    }
}

impl ProtocolEncoding for Signer {
    type Err = Error;

    fn pre_encoding(&self) -> (MessageVersion, Vec<u8>) {
        let proposal: ProtoSigner = self.into();
        (MessageVersion::ProtoBuf, proposal.encode_to_vec())
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
