// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

#[cfg(all(target_arch = "wasm32", feature = "blocking"))]
compile_error!("`blocking` feature can't be enabled for WASM targets");

#[macro_use]
extern crate serde;
pub extern crate nostr_sdk;

pub use keechain_core::*;

pub mod client;
pub mod constants;
#[cfg(not(target_arch = "wasm32"))]
mod keychain;
pub mod policy;
pub mod proposal;
pub mod util;

#[cfg(feature = "blocking")]
pub use self::client::blocking::CoinstrClient;
#[cfg(not(feature = "blocking"))]
pub use self::client::CoinstrClient;
#[cfg(not(target_arch = "wasm32"))]
pub use self::keychain::Coinstr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeeRate {
    /// High: confirm in 1 blocks
    High,
    /// Medium: confirm in 6 blocks
    Medium,
    /// Low: confirm in 12 blocks
    Low,
    /// Target blocks
    Custom(usize),
}

impl FeeRate {
    pub fn target_blocks(&self) -> usize {
        match self {
            Self::High => 1,
            Self::Medium => 6,
            Self::Low => 12,
            Self::Custom(target) => *target,
        }
    }
}
