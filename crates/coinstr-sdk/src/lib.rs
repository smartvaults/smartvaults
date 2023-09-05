// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//#![warn(missing_docs)]
#![warn(rustdoc::bare_urls)]
#![doc = include_str!("../README.md")]

pub use coinstr_core as core;
pub use coinstr_protocol as protocol;
pub use nostr_sdk as nostr;

pub mod client;
pub mod config;
pub mod constants;
pub mod db;
pub mod logger;
pub mod manager;
pub mod prelude;
pub mod types;
pub mod util;

pub use self::client::{Coinstr, EventHandled, Message};
pub use self::types::PolicyBackup;
