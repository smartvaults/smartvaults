// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serializer};
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;

pub(crate) fn serialize_psbt<S>(
    psbt: &PartiallySignedTransaction,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&psbt.to_string())
}

pub(crate) fn deserialize_psbt<'de, D>(
    deserializer: D,
) -> Result<PartiallySignedTransaction, D::Error>
where
    D: Deserializer<'de>,
{
    let psbt = String::deserialize(deserializer)?;
    PartiallySignedTransaction::from_str(&psbt).map_err(::serde::de::Error::custom)
}
