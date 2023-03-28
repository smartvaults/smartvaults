use std::env;
use std::path::PathBuf;
use std::str::FromStr;

use coinstr_core::Result;

pub fn base_path() -> Result<PathBuf> {
    let path = match env::var("COINSTR_PATH").ok() {
        Some(path) => PathBuf::from_str(&path)?,
        None => dirs::home_dir()
            .expect("Imposible to get the HOME dir")
            .join(".coinstr"),
    };
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

pub fn keychains() -> Result<PathBuf> {
    let main_path = base_path()?;
    let path = main_path.join("keychains");
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}
