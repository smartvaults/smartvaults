// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashSet;
use std::str::FromStr;

use nostr::{Event, EventBuilder, Keys};
use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::network::constants::{ParseMagicError, UnknownMagic};
use smartvaults_core::bitcoin::network::Magic;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use thiserror::Error;

use crate::v1::builder::{self, SmartVaultsEventBuilder};
use crate::v1::constants::{KEY_AGENT_VERIFIED, SMARTVAULTS_PUBLIC_KEY};

#[derive(Debug, Error)]
pub enum Error {
    #[error("magic parse: {0}")]
    MagicParse(#[from] ParseMagicError),
    #[error("unknown network magic: {0}")]
    Network(#[from] UnknownMagic),
    #[error("event builder: {0}")]
    EventBuilder(#[from] builder::Error),
    #[error("wrong kind")]
    WrongKind,
    #[error("event not authored by SmartVaults")]
    NotAuthoredBySmartVaults,
    #[error("event identifier (`d` tag) not found")]
    IdentifierNotFound,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedKeyAgents {
    public_keys: HashSet<XOnlyPublicKey>,
    network: Network,
}

impl VerifiedKeyAgents {
    pub fn new(public_keys: HashSet<XOnlyPublicKey>, network: Network) -> Self {
        Self {
            public_keys,
            network,
        }
    }

    pub fn empty(network: Network) -> Self {
        Self::new(HashSet::new(), network)
    }

    pub fn from_event(event: &Event) -> Result<Self, Error> {
        // Check kind
        if event.kind != KEY_AGENT_VERIFIED {
            return Err(Error::WrongKind);
        }

        // Check author
        if event.pubkey.to_string() != SMARTVAULTS_PUBLIC_KEY {
            return Err(Error::NotAuthoredBySmartVaults);
        }

        // Parse and check network magic
        let identifier: &str = event.identifier().ok_or(Error::IdentifierNotFound)?;
        let magic: Magic = Magic::from_str(identifier)?;
        let network: Network = Network::try_from(magic)?;

        // Get public keys
        let public_keys: HashSet<XOnlyPublicKey> = event.public_keys().copied().collect();

        // Compose struct
        Ok(Self::new(public_keys, network))
    }

    pub fn public_keys(&self) -> HashSet<XOnlyPublicKey> {
        self.public_keys.clone()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    /// Check if Key Agent it's verified
    pub fn is_verified(&self, public_key: &XOnlyPublicKey) -> bool {
        self.public_keys.contains(public_key)
    }

    /// Add new verified key agent
    ///
    /// Return `false` if the pubkey already exists
    pub fn add_new_public_key(&mut self, public_key: XOnlyPublicKey) -> bool {
        self.public_keys.insert(public_key)
    }

    /// Remove verified key agent
    ///
    /// Return `false` if the pubkey NOT exists
    pub fn remove_public_key(&mut self, public_key: &XOnlyPublicKey) -> bool {
        self.public_keys.remove(public_key)
    }

    /// Generate [`Event`]
    pub fn to_event(&self, keys: &Keys) -> Result<Event, Error> {
        Ok(EventBuilder::key_agents_verified(
            keys,
            self.public_keys.clone(),
            self.network,
        )?)
    }
}
