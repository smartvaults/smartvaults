// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

use nostr_sdk_ffi::profile::Profile;
pub use smartvaults_sdk::protocol::v1::key_agent::{self, Currency};
use smartvaults_sdk::protocol::v1::BasisPoints;
use smartvaults_sdk::types;
use uniffi::{Enum, Object, Record};

use crate::error::Result;
use crate::Network;

#[derive(Record)]
pub struct KeyAgent {
    pub user: Arc<Profile>,
    pub signer_offerings: Vec<SignerOffering>,
    pub verified: bool,
    pub is_contact: bool,
}

impl From<types::KeyAgent> for KeyAgent {
    fn from(value: types::KeyAgent) -> Self {
        Self {
            user: Arc::new(value.user.into()),
            signer_offerings: value.list.into_iter().map(|s| s.into()).collect(),
            verified: value.verified,
            is_contact: value.is_contact,
        }
    }
}

#[derive(Record)]
pub struct SignerOffering {
    pub temperature: Temperature,
    pub response_time: u16,
    pub device_type: DeviceType,
    pub cost_per_signature: Option<Arc<Price>>,
    pub yearly_cost_basis_points: Option<u64>,
    pub yearly_cost: Option<Arc<Price>>,
    pub network: Network,
}

impl From<key_agent::SignerOffering> for SignerOffering {
    fn from(value: key_agent::SignerOffering) -> Self {
        Self {
            temperature: value.temperature.into(),
            response_time: value.response_time,
            device_type: value.device_type.into(),
            cost_per_signature: value.cost_per_signature.map(|c| Arc::new(c.into())),
            yearly_cost_basis_points: value.yearly_cost_basis_points.map(|p| *p),
            yearly_cost: value.yearly_cost.map(|c| Arc::new(c.into())),
            network: value.network.into(),
        }
    }
}

impl From<SignerOffering> for key_agent::SignerOffering {
    fn from(value: SignerOffering) -> Self {
        Self {
            temperature: value.temperature.into(),
            response_time: value.response_time,
            device_type: value.device_type.into(),
            cost_per_signature: value.cost_per_signature.map(|c| **c),
            yearly_cost_basis_points: value.yearly_cost_basis_points.map(BasisPoints::from),
            yearly_cost: value.yearly_cost.map(|c| **c),
            network: value.network.into(),
        }
    }
}

#[derive(Enum)]
pub enum Temperature {
    Warm,
    Cold,
    AirGapped,
    Unknown,
}

impl From<Temperature> for key_agent::Temperature {
    fn from(value: Temperature) -> Self {
        match value {
            Temperature::Warm => Self::Warm,
            Temperature::Cold => Self::Cold,
            Temperature::AirGapped => Self::AirGapped,
            Temperature::Unknown => Self::Unknown,
        }
    }
}

impl From<key_agent::Temperature> for Temperature {
    fn from(value: key_agent::Temperature) -> Self {
        match value {
            key_agent::Temperature::Warm => Self::Warm,
            key_agent::Temperature::Cold => Self::Cold,
            key_agent::Temperature::AirGapped => Self::AirGapped,
            key_agent::Temperature::Unknown => Self::Unknown,
        }
    }
}

#[derive(Enum)]
pub enum DeviceType {
    Coldcard,
    BitBox02,
    Ledger,
    Mobile,
    Desktop,
    CloudBased,
    Undisclosed,
    Unknown,
}

impl From<DeviceType> for key_agent::DeviceType {
    fn from(value: DeviceType) -> Self {
        match value {
            DeviceType::Coldcard => Self::Coldcard,
            DeviceType::BitBox02 => Self::BitBox02,
            DeviceType::Ledger => Self::Ledger,
            DeviceType::Mobile => Self::Mobile,
            DeviceType::Desktop => Self::Desktop,
            DeviceType::CloudBased => Self::CloudBased,
            DeviceType::Undisclosed => Self::Undisclosed,
            DeviceType::Unknown => Self::Unknown,
        }
    }
}

impl From<key_agent::DeviceType> for DeviceType {
    fn from(value: key_agent::DeviceType) -> Self {
        match value {
            key_agent::DeviceType::Coldcard => Self::Coldcard,
            key_agent::DeviceType::BitBox02 => Self::BitBox02,
            key_agent::DeviceType::Ledger => Self::Ledger,
            key_agent::DeviceType::Mobile => Self::Mobile,
            key_agent::DeviceType::Desktop => Self::Desktop,
            key_agent::DeviceType::CloudBased => Self::CloudBased,
            key_agent::DeviceType::Undisclosed => Self::Undisclosed,
            key_agent::DeviceType::Unknown => Self::Unknown,
        }
    }
}

#[derive(Object)]
pub struct Price {
    inner: key_agent::Price,
}

impl Deref for Price {
    type Target = key_agent::Price;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<key_agent::Price> for Price {
    fn from(inner: key_agent::Price) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl Price {
    /// Compose new price
    ///
    /// Currency must follow ISO 4217 format (3 uppercase chars)
    #[uniffi::constructor]
    pub fn new(amount: u64, currency: String) -> Result<Self> {
        Ok(Self {
            inner: key_agent::Price {
                amount,
                currency: Currency::from_str(&currency)?,
            },
        })
    }

    pub fn amount(&self) -> u64 {
        self.inner.amount
    }

    pub fn currency(&self) -> String {
        self.inner.currency.to_string()
    }
}
