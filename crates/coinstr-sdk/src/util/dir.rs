// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::path::{Path, PathBuf};

use coinstr_core::bitcoin::{Network, XOnlyPublicKey};
use coinstr_core::util::dir;
pub use coinstr_core::util::dir::Error;

fn network_path<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = base_path.as_ref().join(network.to_string());
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

fn keychains_path<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = network_path(base_path, network)?.join("keychains");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

fn cache_path<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = network_path(base_path, network)?.join("cache");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

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
    let path = cache_path(base_path, network)?.join("users");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path.join(format!("{public_key}.db")))
}

pub(crate) fn timechain_db<P>(base_path: P, network: Network) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = cache_path(base_path, network)?.join("timechain");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

pub(crate) fn get_keychains_list<P>(base_path: P, network: Network) -> Result<Vec<String>, Error>
where
    P: AsRef<Path>,
{
    let keychains_path = keychains_path(base_path, network)?;
    dir::get_keychains_list(keychains_path)
}

pub(crate) fn get_keychain_file<P, S>(
    base_path: P,
    network: Network,
    name: S,
) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
    S: Into<String>,
{
    let keychains_path = keychains_path(base_path, network)?;
    dir::get_keychain_file(keychains_path, name)
}
