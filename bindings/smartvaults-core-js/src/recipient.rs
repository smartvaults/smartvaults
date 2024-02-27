// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::str::FromStr;

use smartvaults_core::bitcoin::{Address, Amount};
use smartvaults_core::{Destination, Recipient};
use wasm_bindgen::prelude::*;

use crate::error::{into_err, Result};
use crate::network::JsNetwork;

#[derive(Clone, Copy)]
#[wasm_bindgen(js_name = Amount)]
pub struct JsAmount {
    inner: Amount,
}

impl Deref for JsAmount {
    type Target = Amount;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<Amount> for JsAmount {
    fn from(inner: Amount) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = Amount)]
impl JsAmount {
    #[wasm_bindgen(js_name = fromSat)]
    pub fn from_sat(satoshi: u64) -> Self {
        Self {
            inner: Amount::from_sat(satoshi),
        }
    }

    #[wasm_bindgen(js_name = fromBtc)]
    pub fn from_btc(btc: f64) -> Result<JsAmount> {
        Ok(Self {
            inner: Amount::from_btc(btc).map_err(into_err)?,
        })
    }

    #[wasm_bindgen(js_name = toSat)]
    pub fn to_sat(&self) -> u64 {
        self.inner.to_sat()
    }

    #[wasm_bindgen(js_name = toBtc)]
    pub fn to_btc(&self) -> f64 {
        self.inner.to_btc()
    }
}

#[wasm_bindgen(js_name = Address)]
pub struct JsAddress {
    inner: Address,
}

impl Deref for JsAddress {
    type Target = Address;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<Address> for JsAddress {
    fn from(inner: Address) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = Address)]
impl JsAddress {
    pub fn parse(address: &str, network: JsNetwork) -> Result<JsAddress> {
        Ok(Self {
            inner: Address::from_str(address)
                .map_err(into_err)?
                .require_network(network.into())
                .map_err(into_err)?,
        })
    }

    #[wasm_bindgen(js_name = asStr)]
    pub fn as_str(&self) -> String {
        self.inner.to_string()
    }

    #[wasm_bindgen(js_name = asQrUri)]
    pub fn as_qr_uri(&self) -> String {
        self.inner.to_qr_uri()
    }
}

/// Address recipient
#[wasm_bindgen(js_name = Recipient)]
pub struct JsRecipient {
    inner: Recipient,
}

#[wasm_bindgen(js_class = Recipient)]
impl JsRecipient {
    pub fn new(address: &JsAddress, amount: JsAmount) -> Self {
        Self {
            inner: Recipient {
                address: address.inner.clone(),
                amount: *amount,
            },
        }
    }

    pub fn address(&self) -> JsAddress {
        self.inner.address.clone().into()
    }

    pub fn amount(&self) -> JsAmount {
        self.inner.amount.into()
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
    pub fn drain(address: &JsAddress) -> Self {
        Self {
            inner: Destination::Drain(address.inner.clone()),
        }
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
