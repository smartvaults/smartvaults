// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;

use serde::{Deserialize, Deserializer, Serialize};
use smartvaults_core::bitcoin::consensus;
use smartvaults_core::bitcoin::network::constants::UnknownMagic;
use smartvaults_core::bitcoin::network::Magic;
use smartvaults_core::bitcoin::Network;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ConsensusEncode(#[from] consensus::encode::Error),
    #[error(transparent)]
    Magic(#[from] UnknownMagic),
}

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
    pub fn from_slice(slice: &[u8]) -> Result<Self, Error> {
        let magic: Magic = consensus::deserialize(slice)?;
        Ok(Self {
            inner: Network::try_from(magic)?,
        })
    }
}

impl Serialize for NetworkMagic {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let magic: [u8; 4] = self.magic().to_bytes();
        serializer.serialize_bytes(&magic)
    }
}

impl<'de> Deserialize<'de> for NetworkMagic {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        type Bytes = [u8; 4];
        let bytes: Bytes = Bytes::deserialize(deserializer)?;
        let magic: Magic = Magic::from_bytes(bytes);
        let network: Network = Network::try_from(magic).map_err(serde::de::Error::custom)?;
        Ok(Self { inner: network })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_magic_serialization() {
        let magic = NetworkMagic::from(Network::Bitcoin);
        let ser: Vec<u8> = serde_json::to_vec(&magic).unwrap();
        assert_eq!(&ser, b"[249,190,180,217]");
    }

    #[test]
    fn test_network_magic_deserialization() {
        let magic: &str = "[249,190,180,217]";
        let network: NetworkMagic = serde_json::from_slice(magic.as_bytes()).unwrap();
        assert_eq!(network, Network::Bitcoin.into());

        let magic: &str = "[11,17,9,7]";
        let network: NetworkMagic = serde_json::from_slice(magic.as_bytes()).unwrap();
        assert_eq!(network, Network::Testnet.into());
    }
}
