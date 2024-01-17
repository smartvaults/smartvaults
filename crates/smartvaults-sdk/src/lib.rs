// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

#![forbid(unsafe_code)]
//#![warn(missing_docs)]
#![warn(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]

pub use {nostr_sdk as nostr, smartvaults_core as core, smartvaults_protocol as protocol};

pub mod client;
pub mod config;
pub mod constants;
mod error;
pub mod logger;
pub mod manager;
pub mod prelude;
mod storage;
pub mod types;
pub mod util;

pub use self::client::{EventHandled, Message, SmartVaults};
pub use self::error::Error;
pub use self::types::PolicyBackup;

pub fn git_hash_version() -> Option<String> {
    std::env::var("GIT_HASH").ok()
}
