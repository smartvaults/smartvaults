// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fs::File;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use bdk::bitcoin::Network;
use coinstr_core::util;
use nostr_sdk::Url;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::util::dir;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Dir(#[from] dir::Error),
    #[error(transparent)]
    Json(#[from] nostr_sdk::serde_json::Error),
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

impl From<&Config> for ConfigFile {
    fn from(config: &Config) -> Self {
        Self {
            bitcoin: BitcoinFile {
                electrum_server: (*config.bitcoin.electrum_server.read()).clone(),
                proxy: *config.bitcoin.proxy.read(),
                block_explorer: (*config.bitcoin.block_explorer.read()).clone(),
            },
        }
    }
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
                Err(e) => log::error!("Impossible to deserialize config file: {e}"),
            };
        }

        log::warn!("Using default config");

        let endpoint = match network {
            Network::Bitcoin => "ssl://blockstream.info:700",
            Network::Testnet => "ssl://blockstream.info:993",
            Network::Signet => "tcp://signet-electrumx.wakiyamap.dev:50001",
            Network::Regtest => "tcp://localhost:60401",
        };

        Ok(Self {
            config_file_path,
            bitcoin: Bitcoin {
                electrum_server: Arc::new(RwLock::new(Some(endpoint.to_string()))),
                ..Default::default()
            },
        })
    }

    pub fn save(&self) -> Result<(), Error> {
        let config_file: ConfigFile = self.into();
        let data: Vec<u8> = util::serde::serialize(config_file)?;
        let mut file: File = File::options()
            .create(true)
            .write(true)
            .truncate(true)
            .open(self.config_file_path.as_path())?;
        file.write_all(&data)?;
        Ok(())
    }

    pub fn set_electrum_endpoint<S>(&self, endpoint: Option<S>)
    where
        S: Into<String>,
    {
        let mut e = self.bitcoin.electrum_server.write();
        *e = endpoint.map(|e| e.into());
    }

    pub fn electrum_endpoint(&self) -> Result<String, Error> {
        let endpoint = self.bitcoin.electrum_server.read();
        endpoint.clone().ok_or(Error::ElectrumEndpointNotSet)
    }

    pub fn set_proxy(&self, proxy: Option<SocketAddr>) {
        let mut e = self.bitcoin.proxy.write();
        *e = proxy;
    }

    pub fn proxy(&self) -> Result<SocketAddr, Error> {
        let proxy = self.bitcoin.proxy.read();
        (*proxy).ok_or(Error::ProxyNotSet)
    }

    pub fn set_block_explorer(&self, url: Option<Url>) {
        let mut e = self.bitcoin.block_explorer.write();
        *e = url;
    }

    pub fn block_explorer(&self) -> Result<Url, Error> {
        let block_explorer = self.bitcoin.block_explorer.read();
        block_explorer.clone().ok_or(Error::BlockExplorerNotSet)
    }

    pub fn as_pretty_json(&self) -> Result<String, Error> {
        let config_file: ConfigFile = self.into();
        Ok(nostr_sdk::serde_json::to_string_pretty(&config_file)?)
    }
}
