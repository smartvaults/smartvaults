// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::process::Command;

fn main() {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(output.stdout).unwrap();
    let version = format!("v{}  {}", env!("CARGO_PKG_VERSION"), git_hash);
    println!("cargo:rustc-env=CARGO_PKG_VERSION={version}");
}
