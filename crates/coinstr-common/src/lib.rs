// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::env;
use std::io::Error;
use std::path::{Path, PathBuf};

pub fn base_path() -> Result<PathBuf, Error> {
    let path = match env::var("COINSTR_PATH").ok() {
        Some(path) => Path::new(&path).to_path_buf(),
        None => dirs::home_dir()
            .expect("Imposible to get the HOME dir")
            .join(".coinstr"),
    };
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}
