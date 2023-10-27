// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashSet;
use std::str::FromStr;

use nostr::{Event, Tag};
use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::network::constants::{ParseMagicError, UnknownMagic};
use smartvaults_core::bitcoin::network::Magic;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use thiserror::Error;

use crate::v1::constants::{KEY_AGENT_VERIFIED, SMARTVAULTS_PUBLIC_KEY};

#[derive(Debug, Error)]
pub enum Error {
    #[error("magic parse: {0}")]
    MagicParse(#[from] ParseMagicError),
    #[error("unknown network magic: {0}")]
    Network(#[from] UnknownMagic),
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
    pub fn from_event(event: &Event) -> Result<Self, Error> {
        if event.kind != KEY_AGENT_VERIFIED {
            return Err(Error::WrongKind);
        }

        if event.pubkey.to_string() != SMARTVAULTS_PUBLIC_KEY {
            return Err(Error::NotAuthoredBySmartVaults);
        }

        let identifier: &str = event.identifier().ok_or(Error::IdentifierNotFound)?;
        let magic: Magic = Magic::from_str(identifier)?;
        let network: Network = Network::try_from(magic)?;

        // TODO: use event.public_keys()
        let mut public_keys: HashSet<XOnlyPublicKey> = HashSet::new();
        for tag in event.tags.iter() {
            if let Tag::PubKey(public_key, ..) = tag {
                public_keys.insert(*public_key);
            }
        }

        Ok(Self {
            public_keys,
            network,
        })
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
}
