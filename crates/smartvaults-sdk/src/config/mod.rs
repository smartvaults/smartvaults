// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::fs::File;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use nostr_sdk::Url;
use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::Network;
use smartvaults_core::util;
use thiserror::Error;
use tokio::sync::RwLock;

use crate::util::dir;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Dir(#[from] dir::Error),
    #[error(transparent)]
    Json(#[from] nostr_sdk::serde_json::Error),
    #[error(transparent)]
    Url(#[from] nostr_sdk::url::ParseError),
    #[error("electrum endpoint not set")]
    ElectrumEndpointNotSet,
    #[error("proxy not set")]
    ProxyNotSet,
    #[error("block explorer not set")]
    BlockExplorerNotSet,
}

#[derive(Serialize, Deserialize)]
struct BitcoinFile {
    electrum_server: Option<String>,
    proxy: Option<SocketAddr>,
    block_explorer: Option<Url>,
}

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    bitcoin: BitcoinFile,
}

#[derive(Debug, Clone, Default)]
pub struct Bitcoin {
    pub electrum_server: Arc<RwLock<Option<String>>>,
    pub proxy: Arc<RwLock<Option<SocketAddr>>>,
    pub block_explorer: Arc<RwLock<Option<Url>>>,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub config_file_path: PathBuf,
    pub bitcoin: Bitcoin,
}

impl Config {
    /// Try to get config from file, otherwise will return the default configs
    pub fn try_from_file<P>(base_path: P, network: Network) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let base_path: PathBuf = base_path.as_ref().to_path_buf();
        let config_file_path: PathBuf = dir::config_file_path(base_path, network)?;

        if config_file_path.exists() {
            let mut file: File = File::open(config_file_path.as_path())?;
            let mut content: Vec<u8> = Vec::new();
            file.read_to_end(&mut content)?;

            match util::serde::deserialize::<ConfigFile>(content) {
                Ok(config_file) => {
                    return Ok(Self {
                        config_file_path,
                        bitcoin: Bitcoin {
                            electrum_server: Arc::new(RwLock::new(
                                config_file.bitcoin.electrum_server,
                            )),
                            proxy: Arc::new(RwLock::new(config_file.bitcoin.proxy)),
                            block_explorer: Arc::new(RwLock::new(
                                config_file.bitcoin.block_explorer,
                            )),
                        },
                    })
                }
                Err(e) => tracing::error!("Impossible to deserialize config file: {e}"),
            };
        }

        tracing::warn!("Using default config");

        let (endpoint, block_explorer) = match network {
            Network::Bitcoin => (
                "ssl://blockstream.info:700",
                Some(Url::parse("https://mempool.space")?),
            ),
            Network::Testnet => (
                "ssl://blockstream.info:993",
                Some(Url::parse("https://mempool.space/testnet")?),
            ),
            Network::Signet => (
                "tcp://signet-electrumx.wakiyamap.dev:50001",
                Some(Url::parse("https://mempool.space/signet")?),
            ),
            _ => ("tcp://localhost:60401", None),
        };

        Ok(Self {
            config_file_path,
            bitcoin: Bitcoin {
                electrum_server: Arc::new(RwLock::new(Some(endpoint.to_string()))),
                block_explorer: Arc::new(RwLock::new(block_explorer)),
                ..Default::default()
            },
        })
    }

    async fn to_config_file(&self) -> ConfigFile {
        ConfigFile {
            bitcoin: BitcoinFile {
                electrum_server: (*self.bitcoin.electrum_server.read().await).clone(),
                proxy: *self.bitcoin.proxy.read().await,
                block_explorer: (*self.bitcoin.block_explorer.read().await).clone(),
            },
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn save(&self) -> Result<(), Error> {
        let config_file: ConfigFile = self.to_config_file().await;
        let data: Vec<u8> = util::serde::serialize(config_file)?;
        let mut file: File = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.config_file_path.as_path())?;
        file.write_all(&data)?;
        Ok(())
    }

    pub async fn set_electrum_endpoint<S>(&self, endpoint: Option<S>)
    where
        S: Into<String>,
    {
        let mut e = self.bitcoin.electrum_server.write().await;
        *e = endpoint.map(|e| e.into());
    }

    pub async fn electrum_endpoint(&self) -> Result<String, Error> {
        let endpoint = self.bitcoin.electrum_server.read().await;
        endpoint.clone().ok_or(Error::ElectrumEndpointNotSet)
    }

    pub async fn set_proxy(&self, proxy: Option<SocketAddr>) {
        let mut e = self.bitcoin.proxy.write().await;
        *e = proxy;
    }

    pub async fn proxy(&self) -> Result<SocketAddr, Error> {
        let proxy = self.bitcoin.proxy.read().await;
        (*proxy).ok_or(Error::ProxyNotSet)
    }

    pub async fn set_block_explorer(&self, url: Option<Url>) {
        let mut e = self.bitcoin.block_explorer.write().await;
        *e = url;
    }

    pub async fn block_explorer(&self) -> Result<Url, Error> {
        let block_explorer = self.bitcoin.block_explorer.read().await;
        block_explorer.clone().ok_or(Error::BlockExplorerNotSet)
    }

    pub async fn as_pretty_json(&self) -> Result<String, Error> {
        let config_file: ConfigFile = self.to_config_file().await;
        Ok(nostr_sdk::serde_json::to_string_pretty(&config_file)?)
    }
}
