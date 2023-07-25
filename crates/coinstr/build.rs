// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::process::Command;

fn main() {
    if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output() {
        if let Ok(git_hash) = String::from_utf8(output.stdout) {
            println!("cargo:rustc-env=GIT_HASH={git_hash}");
        }
    }
}
