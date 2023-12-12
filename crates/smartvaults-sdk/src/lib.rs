// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//#![warn(missing_docs)]
#![warn(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]

pub use nostr_sdk as nostr;
pub use smartvaults_core as core;
pub use smartvaults_protocol as protocol;

pub mod client;
pub mod config;
pub mod constants;
mod error;
pub mod logger;
mod manager;
pub mod prelude;
pub mod types;
pub mod util;

pub use self::client::{EventHandled, Message, SmartVaults};
pub use self::error::Error;
pub use self::types::PolicyBackup;

pub fn git_hash_version() -> &'static str {
    env!("GIT_HASH")
}
