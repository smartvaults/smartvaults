// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::str::FromStr;

use smartvaults_core::bitcoin::{Address, Amount};
use smartvaults_core::{Destination, Recipient};
use wasm_bindgen::prelude::*;

use crate::error::{into_err, Result};
use crate::network::JsNetwork;

/// Address recipient
#[wasm_bindgen(js_name = Recipient)]
pub struct JsRecipient {
    inner: Recipient,
}

#[wasm_bindgen(js_class = Recipient)]
impl JsRecipient {
    pub fn new(address: &str, network: JsNetwork, satoshi: u64) -> Result<JsRecipient> {
        let address: Address = Address::from_str(address)
            .map_err(into_err)?
            .require_network(network.into())
            .map_err(into_err)?;
        Ok(Self {
            inner: Recipient {
                address,
                amount: Amount::from_sat(satoshi),
            },
        })
    }

    pub fn address(&self) -> String {
        self.inner.address.to_string()
    }

    pub fn amount(&self) -> u64 {
        self.inner.amount.to_sat()
    }
}

#[wasm_bindgen(js_name = Destination)]
pub struct JsDestination {
    inner: Destination,
}

impl Deref for JsDestination {
    type Target = Destination;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[wasm_bindgen(js_class = Destination)]
impl JsDestination {
    /// Drain all funds to an address
    pub fn drain(address: &str, network: JsNetwork) -> Result<JsDestination> {
        let address: Address = Address::from_str(address)
            .map_err(into_err)?
            .require_network(network.into())
            .map_err(into_err)?;
        Ok(Self {
            inner: Destination::Drain(address),
        })
    }

    pub fn single(recipient: &JsRecipient) -> Self {
        Self {
            inner: Destination::Single(recipient.inner.clone()),
        }
    }

    pub fn multiple(recipients: Vec<JsRecipient>) -> Self {
        Self {
            inner: Destination::Multiple(recipients.into_iter().map(|r| r.inner).collect()),
        }
    }
}
