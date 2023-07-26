// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

mod abortable;
mod amount;
mod approval;
mod balance;
mod client;
mod config;
mod error;
mod key;
mod logger;
mod metadata;
mod nip46;
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
    pub use nostr_ffi::{NostrConnectURI, Timestamp};

    // Namespace
    pub use crate::get_keychains_list;
    pub use crate::logger::{init_logger, LogLevel};

    // Coinstr
    pub use crate::abortable::AbortHandle;
    pub use crate::amount::Amount;
    pub use crate::approval::Approval;
    pub use crate::balance::Balance;
    pub use crate::client::{Coinstr, SyncHandler};
    pub use crate::config::Config;
    pub use crate::key::Keys;
    pub use crate::metadata::Metadata;
    pub use crate::nip46::{NostrConnectRequest, NostrConnectSession};
    pub use crate::policy::{GetPolicy, Policy};
    pub use crate::proposal::{
        ApprovedProposal, CompletedProposal, GetCompletedProposal, GetProposal, Proposal,
    };
    pub use crate::relay::Relay;
    pub use crate::seed::Seed as KeychainSeed;
    pub use crate::signer::Signer;
    pub use crate::transaction::{BlockTime, TransactionDetails};
    pub use crate::utxo::{OutPoint, Utxo};

    // UDL
    uniffi_macros::include_scaffolding!("coinstr_sdk");
}
pub use ffi::*;
