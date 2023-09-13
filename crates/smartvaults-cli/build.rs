// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::process::Command;

fn main() {
    if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        if let Ok(git_hash) = String::from_utf8(output.stdout) {
            let version = format!("v{}  {}", env!("CARGO_PKG_VERSION"), git_hash);
            println!("cargo:rustc-env=CARGO_PKG_VERSION={version}");
        }
    }
}
