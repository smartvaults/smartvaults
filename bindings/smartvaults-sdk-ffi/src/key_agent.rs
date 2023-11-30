// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::str::FromStr;
use std::sync::Arc;

pub use smartvaults_sdk::protocol::v1::key_agent::{self, Currency, DeviceType, Temperature};
use smartvaults_sdk::protocol::v1::BasisPoints;
use smartvaults_sdk::types;

use crate::error::Result;
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
    pub cost_per_signature: Option<Arc<Price>>,
    pub yearly_cost_basis_points: Option<u64>,
    pub yearly_cost: Option<Arc<Price>>,
    pub network: Network,
}

impl From<key_agent::SignerOffering> for SignerOffering {
    fn from(value: key_agent::SignerOffering) -> Self {
        Self {
            temperature: value.temperature,
            response_time: value.response_time,
            device_type: value.device_type,
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
            temperature: value.temperature,
            response_time: value.response_time,
            device_type: value.device_type,
            cost_per_signature: value.cost_per_signature.map(|c| **c),
            yearly_cost_basis_points: value.yearly_cost_basis_points.map(BasisPoints::from),
            yearly_cost: value.yearly_cost.map(|c| **c),
            network: value.network.into(),
        }
    }
}

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

impl Price {
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
