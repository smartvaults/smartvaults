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
    /// Invalid format
    #[error("invalid format")]
    InvalidFormat,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifiedKeyAgents {
    pub names: HashMap<String, XOnlyPublicKey>,
    // pub relays: HashMap<XOnlyPublicKey, Vec<String>>,
}

impl VerifiedKeyAgents {
    /// Check if user it's verified by [`XOnlyPublicKey`] and `smartvaults_nip05` metadata field
    pub fn is_verified(
        &self,
        public_key: &XOnlyPublicKey,
        smartvaults_nip05: &str,
    ) -> Result<bool, Error> {
        let data: Vec<&str> = smartvaults_nip05.split('@').collect();
        if data.len() != 2 {
            return Err(Error::InvalidFormat);
        }
        let name: &str = data[0];
        let domain: &str = data[1];
        if domain == MAINNET_DOMAIN || domain == TESTNET_DOMAIN {
            match self.names.get(name) {
                Some(pk) => Ok(pk == public_key),
                None => Ok(false),
            }
        } else {
            Ok(false)
        }
    }
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
