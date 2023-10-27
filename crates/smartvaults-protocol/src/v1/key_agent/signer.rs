// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::cmp::Ordering;
use core::fmt;
use core::hash::Hash;
use core::num::ParseFloatError;
use core::ops::Deref;
use core::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

use crate::v1::Serde;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SignerOffering {
    /// Temperature
    pub temperature: Temperature,
    /// Response time in minutes
    pub response_time: u16,
    /// Device type
    pub device_type: DeviceType,
    /// Cost per signature
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub cost_per_signature: Option<Price>,
    /// Percentage of the vault balance that should be charged
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub yearly_cost_basis_points: Option<Percentage>,
    /// Yearly cost
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub yearly_cost: Option<Price>,
}

impl Serde for SignerOffering {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Percentage(f64);

impl PartialEq for Percentage {
    fn eq(&self, other: &Self) -> bool {
        self.0.eq(&other.0)
    }
}

impl Eq for Percentage {}

impl PartialOrd for Percentage {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Percentage {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.total_cmp(&other.0)
    }
}

impl Hash for Percentage {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.to_be_bytes().hash(state)
    }
}

impl Deref for Percentage {
    type Target = f64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Percentage {
    pub fn new(p: f64) -> Self {
        Self(p)
    }
}

impl FromStr for Percentage {
    type Err = ParseFloatError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.parse()?))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Temperature {
    Warm,
    Cold,
    AirGapped,
    Other(String),
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warm => write!(f, "warm"),
            Self::Cold => write!(f, "cold"),
            Self::AirGapped => write!(f, "air-gapped"),
            Self::Other(o) => write!(f, "{o}"),
        }
    }
}

impl<S> From<S> for Temperature
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        let value: String = value.into();
        match value.as_str() {
            "warm" => Self::Warm,
            "cold" => Self::Cold,
            "air-gapped" => Self::AirGapped,
            _ => Self::Other(value),
        }
    }
}

impl Serialize for Temperature {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Temperature {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let alphaber: String = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self::from(alphaber))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceType {
    Coldcard,
    BitBox02,
    Ledger,
    Mobile,
    Desktop,
    CloudBased,
    Undisclosed,
    Other(String),
}

impl fmt::Display for DeviceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Coldcard => write!(f, "coldcard"),
            Self::BitBox02 => write!(f, "bitbox02"),
            Self::Ledger => write!(f, "ledger"),
            Self::Mobile => write!(f, "mobile"),
            Self::Desktop => write!(f, "desktop"),
            Self::CloudBased => write!(f, "cloud-based"),
            Self::Undisclosed => write!(f, "undisclosed"),
            Self::Other(o) => write!(f, "{o}"),
        }
    }
}

impl<S> From<S> for DeviceType
where
    S: Into<String>,
{
    fn from(value: S) -> Self {
        let value: String = value.into();
        match value.as_str() {
            "coldcard" => Self::Coldcard,
            "bitbox02" => Self::BitBox02,
            "ledger" => Self::Ledger,
            "mobile" => Self::Mobile,
            "desktop" => Self::Desktop,
            "cloud-based" => Self::CloudBased,
            "undisclosed" => Self::Undisclosed,
            _ => Self::Other(value),
        }
    }
}

impl Serialize for DeviceType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for DeviceType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let alphaber: String = serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self::from(alphaber))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Price {
    pub amount: u64,
    pub currency: String,
}
