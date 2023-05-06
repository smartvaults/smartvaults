// Copyright (c) 2021-2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::fmt;
use std::net::SocketAddr;
use std::path::PathBuf;

use bitcoin::network::constants::Network;
use nostr_sdk::nostr::Keys;
use nostr_sdk::nostr::Url;
use ntfy::Auth;

pub struct Bitcoin {
    pub network: Network,
    pub rpc_addr: SocketAddr,
    pub rpc_username: String,
    pub rpc_password: String,
    pub db_path: PathBuf,
}

#[derive(Deserialize)]
pub struct ConfigFileBitcoin {
    pub network: Option<String>,
    pub rpc_addr: Option<SocketAddr>,
    pub rpc_username: String,
    pub rpc_password: String,
}

pub struct Ntfy {
    pub enabled: bool,
    pub url: String,
    pub topic: String,
    pub auth: Option<Auth>,
    pub proxy: Option<String>,
}

#[derive(Deserialize)]
pub struct ConfigFileNtfy {
    pub enabled: Option<bool>,
    pub url: Option<String>,
    pub topic: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub proxy: Option<String>,
}

pub struct Nostr {
    pub enabled: bool,
    pub keys: Keys,
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub picture: Url,
    pub lud16: String,
    pub relays: Vec<String>,
    pub pow_difficulty: u8,
}

#[derive(Deserialize)]
pub struct ConfigFileNostr {
    pub enabled: Option<bool>,
    pub secret_key: String,
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub picture: Option<Url>,
    pub lud16: Option<String>,
    pub relays: Vec<String>,
    pub pow_difficulty: Option<u8>,
}

pub struct Matrix {
    pub enabled: bool,
    pub homeserver_url: String,
    pub proxy: Option<String>,
    pub user_id: String,
    pub password: String,
    pub admins: Vec<String>,
    pub db_path: PathBuf,
    pub state_path: PathBuf,
}

#[derive(Deserialize)]
pub struct ConfigFileMatrix {
    pub enabled: Option<bool>,
    pub homeserver_url: Option<String>,
    pub proxy: Option<String>,
    pub user_id: Option<String>,
    pub password: Option<String>,
    pub admins: Option<Vec<String>>,
}

#[derive(Debug)]
pub struct Config {
    pub main_path: PathBuf,
    pub log_level: log::Level,
    pub bitcoin: Bitcoin,
    pub ntfy: Ntfy,
    pub nostr: Nostr,
    pub matrix: Matrix,
}

#[derive(Deserialize)]
pub struct ConfigFile {
    pub main_path: Option<PathBuf>,
    pub log_level: Option<String>,
    pub bitcoin: ConfigFileBitcoin,
    pub ntfy: ConfigFileNtfy,
    pub nostr: ConfigFileNostr,
    pub matrix: ConfigFileMatrix,
}

impl fmt::Debug for Bitcoin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ network: {}, rpc_addr: {:?}, rpc_username: {} }}",
            self.network, self.rpc_addr, self.rpc_username
        )
    }
}

impl fmt::Debug for Ntfy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ enabled: {}, url: {:?}, topic: {}, credentials: {}, proxy: {:?} }}",
            self.enabled,
            self.url,
            self.topic,
            self.auth.is_some(),
            self.proxy
        )
    }
}

impl fmt::Debug for Nostr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ enabled: {}, relays: {:?}, pow_difficulty: {} }}",
            self.enabled, self.relays, self.pow_difficulty
        )
    }
}

impl fmt::Debug for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{{ enabled: {}, homeserver_url: {}, proxy: {:?}, user_id: {}, admins: {:?} }}",
            self.enabled, self.homeserver_url, self.proxy, self.user_id, self.admins
        )
    }
}
