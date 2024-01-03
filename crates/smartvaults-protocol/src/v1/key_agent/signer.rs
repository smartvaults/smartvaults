// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::fmt;
use core::hash::Hash;
use core::num::{ParseFloatError, ParseIntError};
use core::ops::Deref;
use core::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;
use smartvaults_core::bitcoin::Network;
use thiserror::Error;

use crate::v1::network::{deserialize_network, serialize_network};
use crate::v1::Serde;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    ParseInt(#[from] ParseIntError),
    #[error(transparent)]
    ParseFloat(#[from] ParseFloatError),
    #[error("invalid price")]
    InvalidPrice,
    #[error("invalid currency: must follow ISO 4217 format (3 uppercase chars)")]
    InvalidCurrency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct SignerOffering {
    /// Temperature
    #[serde(default)]
    pub temperature: Temperature,
    /// Response time in minutes
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub response_time: Option<u16>,
    /// Device type
    #[serde(default)]
    pub device_type: DeviceType,
    /// Cost per signature
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub cost_per_signature: Option<Price>,
    /// BasisPoints of the vault balance that should be charged
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub yearly_cost_basis_points: Option<BasisPoints>,
    /// Yearly cost
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub yearly_cost: Option<Price>,
    /// Network
    #[serde(
        serialize_with = "serialize_network",
        deserialize_with = "deserialize_network"
    )]
    pub network: Network,
}

impl Serde for SignerOffering {}

#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub struct BasisPoints(u64);

impl fmt::Display for BasisPoints {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for BasisPoints {
    type Target = u64;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl BasisPoints {
    pub fn new(basis_points: u64) -> Self {
        Self::from(basis_points)
    }

    /// Compose [`BasisPoints`] from percentage (%)
    ///
    /// # Example
    /// ```rust,no_run
    /// use smartvaults_protocol::v1::BasisPoints;
    ///
    /// let percentage = 2.5; // 2.5%
    /// let _basis_points = BasisPoints::from_percentage(percentage);
    /// ```
    pub fn from_percentage(percentage: f64) -> Self {
        Self::new((percentage * 100.0).round() as u64)
    }
}

impl From<u64> for BasisPoints {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl FromStr for BasisPoints {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::new(s.parse()?))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Temperature {
    Warm,
    Cold,
    AirGapped,
    #[default]
    Unknown,
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Warm => write!(f, "warm"),
            Self::Cold => write!(f, "cold"),
            Self::AirGapped => write!(f, "air-gapped"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl<S> From<S> for Temperature
where
    S: AsRef<str>,
{
    fn from(temp: S) -> Self {
        match temp.as_ref() {
            "warm" => Self::Warm,
            "cold" => Self::Cold,
            "air-gapped" => Self::AirGapped,
            _ => Self::Unknown,
        }
    }
}

impl Temperature {
    pub fn list() -> Vec<Self> {
        vec![Self::Warm, Self::Cold, Self::AirGapped]
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
        let temperature: String =
            serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self::from(&temperature))
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DeviceType {
    Coldcard,
    BitBox02,
    Ledger,
    Mobile,
    Desktop,
    CloudBased,
    Undisclosed,
    #[default]
    Unknown,
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
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

impl<S> From<S> for DeviceType
where
    S: AsRef<str>,
{
    fn from(device_type: S) -> Self {
        match device_type.as_ref() {
            "coldcard" => Self::Coldcard,
            "bitbox02" => Self::BitBox02,
            "ledger" => Self::Ledger,
            "mobile" => Self::Mobile,
            "desktop" => Self::Desktop,
            "cloud-based" => Self::CloudBased,
            "undisclosed" => Self::Undisclosed,
            _ => Self::Unknown,
        }
    }
}

impl DeviceType {
    pub fn list() -> Vec<Self> {
        vec![
            Self::Coldcard,
            Self::BitBox02,
            Self::Ledger,
            Self::Mobile,
            Self::Desktop,
            Self::CloudBased,
            Self::Undisclosed,
        ]
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
        let decive_type: String =
            serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Ok(Self::from(decive_type))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Currency(char, char, char);

impl fmt::Display for Currency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{}{}",
            self.0.to_uppercase(),
            self.1.to_uppercase(),
            self.2.to_uppercase()
        )
    }
}

impl FromStr for Currency {
    type Err = Error;
    fn from_str(currency: &str) -> Result<Self, Self::Err> {
        if currency.len() == 3 {
            let mut chars = currency.chars();
            if let (Some(c1), Some(c2), Some(c3)) = (chars.next(), chars.next(), chars.next()) {
                if c1.is_uppercase() && c2.is_uppercase() && c3.is_uppercase() {
                    Ok(Self(c1, c2, c3))
                } else {
                    Err(Error::InvalidCurrency)
                }
            } else {
                Err(Error::InvalidCurrency)
            }
        } else {
            Err(Error::InvalidCurrency)
        }
    }
}

impl Serialize for Currency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Currency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let decive_type: String =
            serde_json::from_value(value).map_err(serde::de::Error::custom)?;
        Self::from_str(&decive_type).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Price {
    pub amount: u64,
    pub currency: Currency,
}

impl fmt::Display for Price {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.amount, self.currency)
    }
}

impl FromStr for Price {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut split = s.split(' ');
        if let (Some(amount_str), Some(currency)) = (split.next(), split.next()) {
            Ok(Self {
                amount: amount_str.parse()?,
                currency: currency.parse()?,
            })
        } else {
            Err(Error::InvalidPrice)
        }
    }
}
