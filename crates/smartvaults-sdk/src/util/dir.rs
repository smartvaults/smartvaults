// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::path::{Path, PathBuf};

use smartvaults_core::bitcoin::Network;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::util::dir;
pub use smartvaults_core::util::dir::Error;

fn network_path<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = base_path.as_ref().join(network.to_string());
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

pub(crate) fn keychains_path<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = network_path(base_path, network)?.join("keychains");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

/* fn cache_path<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = network_path(base_path, network)?.join("cache");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
} */

pub(crate) fn config_file_path<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    Ok(network_path(base_path, network)?.join("config.json"))
}

pub(crate) fn logs_path<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = network_path(base_path, network)?.join("logs");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

pub(crate) fn user_db<P>(
    base_path: P,
    network: Network,
    public_key: XOnlyPublicKey,
) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = network_path(base_path, network)?.join("users");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path.join(format!("{public_key}.db"))) // TODO: update extension to `sqlite3` if needed a breaking change in DB migrations
}

pub(crate) fn nostr_db<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = network_path(base_path, network)?;
    Ok(path.join("nostr.db"))
}

pub(crate) fn get_keychains_list<P>(base_path: P, network: Network) -> Result<Vec<String>, Error>
where
    P: AsRef<Path>,
{
    let keychains_path = keychains_path(base_path, network)?;
    dir::get_keychains_list(keychains_path)
}
