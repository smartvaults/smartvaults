// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::path::{Path, PathBuf};

use keechain_core::util::dir;
pub use keechain_core::util::dir::Error;
use nostr_sdk::Keys;

fn keychains<P>(base_path: P) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = base_path.as_ref().join("keychains");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

pub(crate) fn nostr_db<P>(base_path: P, keys: &Keys) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = base_path.as_ref().join("nostr");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path.join(format!("nostr-{}.db", keys.public_key())))
}

pub(crate) fn timechain_db<P>(base_path: P) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
{
    let path = base_path.as_ref().join("timechain");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

pub fn get_keychains_list<P>(base_path: P) -> Result<Vec<String>, Error>
where
    P: AsRef<Path>,
{
    let keychains_path = keychains(base_path)?;
    dir::get_keychains_list(keychains_path)
}

pub fn get_keychain_file<P, S>(base_path: P, name: S) -> Result<PathBuf, Error>
where
    P: AsRef<Path>,
    S: Into<String>,
{
    let keychains_path = keychains(base_path)?;
    dir::get_keychain_file(keychains_path, name)
}
