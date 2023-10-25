// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashMap;

use nostr::secp256k1::XOnlyPublicKey;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::Network;
use thiserror::Error;

const MAINNET_DOMAIN: &str = "https://smartvaults.app";
const TESTNET_DOMAIN: &str = "https://test.smartvaults.app";

#[derive(Debug, Error)]
pub enum Error {
    /// Reqwest error
    #[error(transparent)]
    Reqwest(#[from] reqwest::Error),
    /// Error deserializing JSON data
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedKeyAgents {
    pub names: HashMap<String, XOnlyPublicKey>,
    // pub relays: HashMap<XOnlyPublicKey, Vec<String>>,
}

pub async fn get_verified_key_agents(network: Network) -> Result<VerifiedKeyAgents, Error> {
    // Compose URL
    let base_url: &str = match network {
        Network::Bitcoin => MAINNET_DOMAIN,
        _ => TESTNET_DOMAIN,
    };
    let url = format!("{base_url}/.well-known/smartvaults.json");

    // Get JSON
    let client: Client = Client::new();
    let res = client.get(url).send().await?;
    Ok(res.json().await?)
}
