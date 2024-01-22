// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::fmt;
use std::fs::File;
use std::io::{Read, Write};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::str::FromStr;
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
    Url(#[from] nostr_sdk::types::url::ParseError),
    #[error("Invalid electrum endpoint: {0}")]
    InvalidElectrumUrl(String),
    #[error("electrum endpoint not set")]
    ElectrumEndpointNotSet,
    #[error("proxy not set")]
    ProxyNotSet,
    #[error("block explorer not set")]
    BlockExplorerNotSet,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElectrumEndpoint {
    Tls {
        host: String,
        port: u16,
        validate_tls: bool,
    },
    Plaintext {
        host: String,
        port: u16,
    },
}

impl ElectrumEndpoint {
    /// Format: `<host>:<port>:<t|s>`
    ///
    /// Optionally, `:noverify` suffix to skip TLS validation
    pub fn as_standard_format(&self) -> String {
        match self {
            Self::Tls {
                host,
                port,
                validate_tls: true,
            } => format!("{host}:{port}:s"),
            Self::Tls {
                host,
                port,
                validate_tls: false,
            } => format!("{host}:{port}:s:noverify"),
            Self::Plaintext { host, port } => format!("{host}:{port}:t"),
        }
    }

    /// Format: `<tcp|ssl>://<host>:<port>`
    pub fn as_non_standard_format(&self) -> String {
        match self {
            Self::Tls { host, port, .. } => format!("ssl://{host}:{port}"),
            Self::Plaintext { host, port } => format!("tcp://{host}:{port}"),
        }
    }

    pub fn validate_tls(&self) -> bool {
        match self {
            Self::Tls { validate_tls, .. } => *validate_tls,
            Self::Plaintext { .. } => false,
        }
    }
}

impl fmt::Display for ElectrumEndpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_standard_format())
    }
}

// Parse endpoint with <tcp|ssl>://<host>:<port> format
//
// OR
//
// Parse the standard <host>:<port>:<t|s> string format.
//
// Both formats support an optional non-standard `:noverify` suffix to skip tls validation
impl FromStr for ElectrumEndpoint {
    type Err = Error;

    fn from_str(endpoint: &str) -> Result<Self, Error> {
        if endpoint.starts_with("ssl://") || endpoint.starts_with("tcp://") {
            // Remove the protocol part
            let without_protocol = endpoint
                .get(6..)
                .ok_or_else(|| Error::InvalidElectrumUrl(String::from("Missing endpoint")))?;

            // Split
            let mut splitted = without_protocol.split(':');
            let host: &str = splitted
                .next()
                .ok_or_else(|| Error::InvalidElectrumUrl(String::from("Missing host")))?;
            let port: u16 = splitted
                .next()
                .ok_or_else(|| Error::InvalidElectrumUrl(String::from("Missing port")))?
                .parse()
                .map_err(|_| Error::InvalidElectrumUrl(String::from("Invalid port")))?;

            if endpoint.starts_with("ssl://") {
                Ok(Self::Tls {
                    host: host.to_string(),
                    port,
                    validate_tls: true,
                })
            } else {
                Ok(Self::Plaintext {
                    host: host.to_string(),
                    port,
                })
            }
        } else {
            let mut splitted = endpoint.split(':');
            let host: &str = splitted
                .next()
                .ok_or_else(|| Error::InvalidElectrumUrl(String::from("Missing host")))?;
            let port: u16 = splitted
                .next()
                .ok_or_else(|| Error::InvalidElectrumUrl(String::from("Missing port")))?
                .parse()
                .map_err(|_| Error::InvalidElectrumUrl(String::from("Invalid port")))?;
            let protocol: &str = splitted.next().unwrap_or("t");
            let validate_tls: bool = splitted.next() != Some("noverify");
            match protocol {
                "s" => Ok(Self::Tls {
                    host: host.to_string(),
                    port,
                    validate_tls,
                }),
                "t" => Ok(Self::Plaintext {
                    host: host.to_string(),
                    port,
                }),
                p => Err(Error::InvalidElectrumUrl(format!("Unknown protocol: {p}"))),
            }
        }
    }
}

impl Serialize for ElectrumEndpoint {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.as_standard_format().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ElectrumEndpoint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let endpoint: String = String::deserialize(deserializer)?;
        Self::from_str(&endpoint).map_err(serde::de::Error::custom)
    }
}

#[derive(Serialize, Deserialize)]
struct BitcoinFile {
    electrum_server: Option<ElectrumEndpoint>,
    proxy: Option<SocketAddr>,
    block_explorer: Option<Url>,
}

#[derive(Serialize, Deserialize)]
struct ConfigFile {
    bitcoin: BitcoinFile,
}

#[derive(Debug, Clone, Default)]
pub struct Bitcoin {
    pub electrum_server: Arc<RwLock<Option<ElectrumEndpoint>>>,
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
                ElectrumEndpoint::Tls {
                    host: String::from("blockstream.info"),
                    port: 700,
                    validate_tls: true,
                },
                Some(Url::parse("https://mempool.space")?),
            ),
            Network::Testnet => (
                ElectrumEndpoint::Tls {
                    host: String::from("blockstream.info"),
                    port: 993,
                    validate_tls: true,
                },
                Some(Url::parse("https://mempool.space/testnet")?),
            ),
            Network::Signet => (
                ElectrumEndpoint::Plaintext {
                    host: String::from("signet-electrumx.wakiyamap.dev"),
                    port: 50001,
                },
                Some(Url::parse("https://mempool.space/signet")?),
            ),
            _ => (
                ElectrumEndpoint::Plaintext {
                    host: String::from("localhost"),
                    port: 60401,
                },
                None,
            ),
        };

        Ok(Self {
            config_file_path,
            bitcoin: Bitcoin {
                electrum_server: Arc::new(RwLock::new(Some(endpoint))),
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

    pub async fn set_electrum_endpoint<S>(&self, endpoint: Option<S>) -> Result<(), Error>
    where
        S: AsRef<str>,
    {
        let mut e = self.bitcoin.electrum_server.write().await;
        match endpoint {
            Some(endpoint) => {
                let endpoint = ElectrumEndpoint::from_str(endpoint.as_ref())?;
                *e = Some(endpoint);
            }
            None => *e = None,
        }
        Ok(())
    }

    pub async fn electrum_endpoint(&self) -> Result<ElectrumEndpoint, Error> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_electrum_endpoint_parse() {
        let endpoint = ElectrumEndpoint::from_str("ssl://blockstream.info:700").unwrap();
        assert_eq!(
            endpoint,
            ElectrumEndpoint::Tls {
                host: String::from("blockstream.info"),
                port: 700,
                validate_tls: true
            }
        );

        let endpoint = ElectrumEndpoint::from_str("blockstream.info:700:s").unwrap();
        assert_eq!(
            endpoint,
            ElectrumEndpoint::Tls {
                host: String::from("blockstream.info"),
                port: 700,
                validate_tls: true
            }
        );

        let endpoint = ElectrumEndpoint::from_str("blockstream.info:993:s:noverify").unwrap();
        assert_eq!(
            endpoint,
            ElectrumEndpoint::Tls {
                host: String::from("blockstream.info"),
                port: 993,
                validate_tls: false
            }
        );

        let endpoint = ElectrumEndpoint::from_str("127.0.0.1:50001").unwrap();
        assert_eq!(
            endpoint,
            ElectrumEndpoint::Plaintext {
                host: String::from("127.0.0.1"),
                port: 50001
            }
        );

        let endpoint = ElectrumEndpoint::from_str("tcp://127.0.0.1:50001").unwrap();
        assert_eq!(
            endpoint,
            ElectrumEndpoint::Plaintext {
                host: String::from("127.0.0.1"),
                port: 50001
            }
        );

        let endpoint = ElectrumEndpoint::from_str("127.0.0.1:50001:t").unwrap();
        assert_eq!(
            endpoint,
            ElectrumEndpoint::Plaintext {
                host: String::from("127.0.0.1"),
                port: 50001
            }
        );

        let endpoint = ElectrumEndpoint::from_str("127.0.0.1:50002:s:noverify").unwrap();
        assert_eq!(
            endpoint,
            ElectrumEndpoint::Tls {
                host: String::from("127.0.0.1"),
                port: 50002,
                validate_tls: false
            }
        );
    }
}
