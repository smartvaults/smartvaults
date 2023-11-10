// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use smartvaults_sdk::protocol::v1::key_agent::signer::Percentage;
pub use smartvaults_sdk::protocol::v1::key_agent::{self, Price};
use smartvaults_sdk::types;

use crate::{Network, User};

pub struct KeyAgent {
    pub user: Arc<User>,
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

pub struct SignerOffering {
    pub temperature: Temperature,
    pub response_time: u16,
    pub device_type: DeviceType,
    pub cost_per_signature: Option<Price>,
    pub yearly_cost_basis_points: Option<f64>,
    pub yearly_cost: Option<Price>,
    pub network: Network,
}

impl From<key_agent::SignerOffering> for SignerOffering {
    fn from(value: key_agent::SignerOffering) -> Self {
        Self {
            temperature: value.temperature.into(),
            response_time: value.response_time,
            device_type: value.device_type.into(),
            cost_per_signature: value.cost_per_signature,
            yearly_cost_basis_points: value.yearly_cost_basis_points.map(|p| *p),
            yearly_cost: value.yearly_cost,
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
            cost_per_signature: value.cost_per_signature,
            yearly_cost_basis_points: value.yearly_cost_basis_points.map(Percentage::new),
            yearly_cost: value.yearly_cost,
            network: value.network.into(),
        }
    }
}

pub enum Temperature {
    Warm(),
    Cold(),
    AirGapped(),
    Other { other: String },
}

impl From<key_agent::Temperature> for Temperature {
    fn from(value: key_agent::Temperature) -> Self {
        match value {
            key_agent::Temperature::Warm => Self::Warm(),
            key_agent::Temperature::Cold => Self::Cold(),
            key_agent::Temperature::AirGapped => Self::AirGapped(),
            key_agent::Temperature::Other(other) => Self::Other { other },
        }
    }
}

impl From<Temperature> for key_agent::Temperature {
    fn from(value: Temperature) -> Self {
        match value {
            Temperature::Warm() => Self::Warm,
            Temperature::Cold() => Self::Cold,
            Temperature::AirGapped() => Self::AirGapped,
            Temperature::Other { other } => Self::Other(other),
        }
    }
}

pub enum DeviceType {
    Coldcard(),
    BitBox02(),
    Ledger(),
    Mobile(),
    Desktop(),
    CloudBased(),
    Undisclosed(),
    Other { other: String },
}

impl From<key_agent::DeviceType> for DeviceType {
    fn from(value: key_agent::DeviceType) -> Self {
        match value {
            key_agent::DeviceType::Coldcard => Self::Coldcard(),
            key_agent::DeviceType::BitBox02 => Self::BitBox02(),
            key_agent::DeviceType::Ledger => Self::Ledger(),
            key_agent::DeviceType::Mobile => Self::Mobile(),
            key_agent::DeviceType::Desktop => Self::Desktop(),
            key_agent::DeviceType::CloudBased => Self::CloudBased(),
            key_agent::DeviceType::Undisclosed => Self::Undisclosed(),
            key_agent::DeviceType::Other(other) => Self::Other { other },
        }
    }
}

impl From<DeviceType> for key_agent::DeviceType {
    fn from(value: DeviceType) -> Self {
        match value {
            DeviceType::Coldcard() => Self::Coldcard,
            DeviceType::BitBox02() => Self::BitBox02,
            DeviceType::Ledger() => Self::Ledger,
            DeviceType::Mobile() => Self::Mobile,
            DeviceType::Desktop() => Self::Desktop,
            DeviceType::CloudBased() => Self::CloudBased,
            DeviceType::Undisclosed() => Self::Undisclosed,
            DeviceType::Other { other } => Self::Other(other),
        }
    }
}
