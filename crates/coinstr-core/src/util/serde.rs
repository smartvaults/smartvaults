// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Error;

pub trait Serde: Sized + Serialize + DeserializeOwned {
    /// Deserialize from `JSON` string
    fn from_json<S>(json: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        Ok(serde_json::from_str(&json.into())?)
    }

    /// Serialize to `JSON` string
    fn as_json(&self) -> String {
        serde_json::json!(self).to_string()
    }
}

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
    PartiallySignedTransaction::from_str(&psbt).map_err(serde::de::Error::custom)
}
