// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serializer};
use smartvaults_core::bitcoin::network::Magic;
use smartvaults_core::bitcoin::Network;

pub(crate) fn serialize_network<S>(network: &Network, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&network.magic().to_string())
}

pub(crate) fn deserialize_network<'de, D>(deserializer: D) -> Result<Network, D::Error>
where
    D: Deserializer<'de>,
{
    let magic = String::deserialize(deserializer)?;
    let magic = Magic::from_str(&magic).map_err(::serde::de::Error::custom)?;
    Network::try_from(magic).map_err(::serde::de::Error::custom)
}
