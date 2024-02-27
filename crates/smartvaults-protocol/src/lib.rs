// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Protocol

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::bare_urls)]
#![cfg_attr(bench, feature(test))]

#[cfg(bench)]
extern crate test;

pub extern crate nostr;
pub use smartvaults_core as core;

pub mod v1;
pub mod v2;
