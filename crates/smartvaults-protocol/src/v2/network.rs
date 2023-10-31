// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Network

use core::ops::Deref;

use smartvaults_core::bitcoin::network::constants::UnknownMagic;
use smartvaults_core::bitcoin::network::Magic;
use smartvaults_core::bitcoin::{consensus, Network};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ConsensusEncode(#[from] consensus::encode::Error),
    #[error(transparent)]
    Magic(#[from] UnknownMagic),
}

/// Network magic
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NetworkMagic {
    inner: Network,
}

impl From<Network> for NetworkMagic {
    fn from(inner: Network) -> Self {
        Self { inner }
    }
}

impl Deref for NetworkMagic {
    type Target = Network;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl NetworkMagic {
    /// Construct from [`Network`]
    pub fn new(network: Network) -> Self {
        Self::from(network)
    }

    /// Construct from bytes
    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        let magic: Magic = consensus::deserialize(slice)?;
        Ok(Self {
            inner: Network::try_from(magic)?,
        })
    }
}
