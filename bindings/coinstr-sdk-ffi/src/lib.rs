// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

mod amount;
mod balance;
mod client;
mod error;
mod key;
mod logger;
mod metadata;
mod policy;
mod proposal;
mod relay;
mod seed;
mod signer;
mod transaction;
mod utxo;

use self::error::Result;

pub fn get_keychains_list(base_path: String, network: Network) -> Result<Vec<String>> {
    Ok(coinstr_sdk::Coinstr::list_keychains(base_path, network)?)
}

mod ffi {
    // Error
    pub use crate::error::FFIError;

    // External
    pub use coinstr_sdk::core::bitcoin::Network;
    pub use coinstr_sdk::core::signer::SignerType;
    pub use coinstr_sdk::core::types::WordCount;
    pub use coinstr_sdk::nostr::RelayStatus;

    // Namespace
    pub use crate::get_keychains_list;
    pub use crate::logger::{init_logger, LogLevel};

    // Coinstr
    pub use crate::amount::Amount;
    pub use crate::balance::Balance;
    pub use crate::client::Coinstr;
    pub use crate::key::Keys;
    pub use crate::metadata::Metadata;
    pub use crate::policy::Policy;
    pub use crate::proposal::{CompletedProposal, Proposal};
    pub use crate::relay::Relay;
    pub use crate::seed::Seed as KeychainSeed;
    pub use crate::signer::Signer;
    pub use crate::transaction::{BlockTime, TransactionDetails};
    pub use crate::utxo::{OutPoint, Utxo};

    // UDL
    uniffi_macros::include_scaffolding!("coinstr_sdk");
}
pub use ffi::*;
