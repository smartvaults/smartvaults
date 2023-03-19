use std::str::FromStr;

use keechain_core::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::Address;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpendingProposal {
    pub memo: String,
    pub to_address: Address,
    pub amount: u64,
    #[serde(
        serialize_with = "serialize_psbt",
        deserialize_with = "deserialize_psbt"
    )]
    pub psbt: PartiallySignedTransaction,
}

impl SpendingProposal {
    pub fn new<S>(
        memo: S,
        to_address: Address,
        amount: u64,
        psbt: PartiallySignedTransaction,
    ) -> Self
    where
        S: Into<String>,
    {
        Self {
            memo: memo.into(),
            to_address,
            amount,
            psbt,
        }
    }

    /// Deserialize from `JSON` string
    pub fn from_json<S>(json: S) -> Result<Self, serde_json::Error>
    where
        S: Into<String>,
    {
        serde_json::from_str(&json.into())
    }

    /// Serialize to `JSON` string
    pub fn as_json(&self) -> String {
        serde_json::json!(self).to_string()
    }
}

fn serialize_psbt<S>(psbt: &PartiallySignedTransaction, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&psbt.to_string())
}

fn deserialize_psbt<'de, D>(deserializer: D) -> Result<PartiallySignedTransaction, D::Error>
where
    D: Deserializer<'de>,
{
    let psbt = String::deserialize(deserializer)?;
    PartiallySignedTransaction::from_str(&psbt).map_err(serde::de::Error::custom)
}
