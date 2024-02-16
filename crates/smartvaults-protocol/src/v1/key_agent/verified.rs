// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashMap;
use std::str::FromStr;

use nostr::{Event, EventBuilder, Keys, PublicKey, Timestamp};
use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::network::constants::{ParseMagicError, UnknownMagic};
use smartvaults_core::bitcoin::network::Magic;
use smartvaults_core::bitcoin::Network;
use thiserror::Error;

use crate::v1::builder::{self, SmartVaultsEventBuilder};
use crate::v1::constants::{
    KEY_AGENT_VERIFIED, SMARTVAULTS_MAINNET_PUBLIC_KEY, SMARTVAULTS_TESTNET_PUBLIC_KEY,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("magic parse: {0}")]
    MagicParse(#[from] ParseMagicError),
    #[error("unknown network magic: {0}")]
    Network(#[from] UnknownMagic),
    #[error("event builder: {0}")]
    EventBuilder(#[from] builder::Error),
    #[error("JSON: {0}")]
    JSON(#[from] serde_json::Error),
    #[error("wrong kind")]
    WrongKind,
    #[error("event not authored by SmartVaults")]
    NotAuthoredBySmartVaults,
    #[error("event identifier (`d` tag) not found")]
    IdentifierNotFound,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedKeyAgentData {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub approved_at: Option<Timestamp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedKeyAgents {
    public_keys: HashMap<PublicKey, VerifiedKeyAgentData>,
    network: Network,
}

impl VerifiedKeyAgents {
    pub fn new(public_keys: HashMap<PublicKey, VerifiedKeyAgentData>, network: Network) -> Self {
        Self {
            public_keys,
            network,
        }
    }

    pub fn empty(network: Network) -> Self {
        Self::new(HashMap::new(), network)
    }

    pub fn from_event(event: &Event) -> Result<Self, Error> {
        // Check kind
        if event.kind != KEY_AGENT_VERIFIED {
            return Err(Error::WrongKind);
        }

        // Parse and check network magic
        let identifier: &str = event.identifier().ok_or(Error::IdentifierNotFound)?;
        let magic: Magic = Magic::from_str(identifier)?;
        let network: Network = Network::try_from(magic)?;

        // Check author
        let authored_by_smartvaults: bool = match network {
            Network::Bitcoin => event.author() == *SMARTVAULTS_MAINNET_PUBLIC_KEY,
            _ => event.author() == *SMARTVAULTS_TESTNET_PUBLIC_KEY,
        };

        if !authored_by_smartvaults {
            return Err(Error::NotAuthoredBySmartVaults);
        }

        // Get public keys
        let public_keys: HashMap<PublicKey, VerifiedKeyAgentData> =
            serde_json::from_str(&event.content)?;

        // Compose struct
        Ok(Self::new(public_keys, network))
    }

    pub fn public_keys(&self) -> HashMap<PublicKey, VerifiedKeyAgentData> {
        self.public_keys.clone()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    /// Check if Key Agent it's verified
    pub fn is_verified(&self, public_key: &PublicKey) -> bool {
        self.public_keys.contains_key(public_key)
    }

    /// Add new verified key agent
    ///
    /// Return `false` if the pubkey already exists
    pub fn add_new_public_key(
        &mut self,
        public_key: PublicKey,
        data: VerifiedKeyAgentData,
    ) -> bool {
        self.public_keys.insert(public_key, data).is_none()
    }

    /// Remove verified key agent
    ///
    /// Return `false` if the pubkey NOT exists
    pub fn remove_public_key(&mut self, public_key: &PublicKey) -> bool {
        self.public_keys.remove(public_key).is_some()
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
