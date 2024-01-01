// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use smartvaults_core::bitcoin::Network;
use wasm_bindgen::prelude::*;

#[wasm_bindgen(js_name = Network)]
pub enum JsNetwork {
    Bitcoin,
    Testnet,
    Signet,
    Regtest,
}

impl From<Network> for JsNetwork {
    fn from(value: Network) -> Self {
        match value {
            Network::Bitcoin => Self::Bitcoin,
            Network::Testnet => Self::Testnet,
            Network::Signet => Self::Signet,
            _ => Self::Regtest,
        }
    }
}

impl From<JsNetwork> for Network {
    fn from(value: JsNetwork) -> Self {
        match value {
            JsNetwork::Bitcoin => Self::Bitcoin,
            JsNetwork::Testnet => Self::Testnet,
            JsNetwork::Signet => Self::Signet,
            JsNetwork::Regtest => Self::Regtest,
        }
    }
}
