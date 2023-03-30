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
