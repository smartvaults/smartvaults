// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Protocol V2

pub mod constants;
mod core;
mod error;
mod network;
pub mod proposal;
mod proto;
pub mod vault;
pub mod wrapper;

pub use self::core::{ProtocolEncoding, ProtocolEncryption};
pub use self::error::Error;
pub use self::network::NetworkMagic;
pub use self::proposal::Proposal;
pub use self::vault::Vault;
use self::wrapper::Wrapper;
