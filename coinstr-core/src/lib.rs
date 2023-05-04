// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

#[macro_use]
extern crate serde;
pub extern crate nostr_sdk;

pub use keechain_core::*;

#[cfg(feature = "cache")]
pub mod cache;
pub mod client;
pub mod constants;
pub mod policy;
pub mod proposal;
pub mod reserves;
pub mod util;

pub use self::client::Coinstr;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum FeeRate {
    /// High: confirm in 1 blocks
    High,
    /// Medium: confirm in 6 blocks
    #[default]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    Max,
    Custom(u64),
}
