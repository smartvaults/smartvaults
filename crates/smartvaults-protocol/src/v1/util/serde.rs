// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use serde::de::DeserializeOwned;
use serde::Serialize;
pub use serde_json::Error;
use smartvaults_core::bdk::descriptor::policy::{Policy as SpendingPolicy, SatisfiableItem};
pub use smartvaults_core::util::serde::*;

pub trait SerdeSer: Sized + Serialize {
    /// Serialize to `JSON` string
    fn as_json(&self) -> String {
        serde_json::json!(self).to_string()
    }
}

pub trait Serde: Sized + Serialize + DeserializeOwned {
    /// Deserialize from `JSON` string
    fn from_json<S>(json: S) -> Result<Self, Error>
    where
        S: Into<String>,
    {
        serde_json::from_str(&json.into())
    }

    /// Serialize to `JSON` string
    fn as_json(&self) -> String {
        serde_json::json!(self).to_string()
    }
}

impl SerdeSer for SpendingPolicy {}
impl SerdeSer for SatisfiableItem {}
