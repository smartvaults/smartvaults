// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

pub use nostr_sdk_ffi::{
    EventId, Keys, Metadata, NostrConnectURI, PublicKey, Relay, RelayConnectionStats,
    RelayInformationDocument, SecretKey, Timestamp,
};
pub use smartvaults_sdk::core::policy::PolicyTemplateType;
pub use smartvaults_sdk::core::signer::SignerType;
pub use smartvaults_sdk::core::types::WordCount;
use smartvaults_sdk::logger;
pub use smartvaults_sdk::nostr::RelayStatus;

mod abortable;
mod address;
mod amount;
mod balance;
mod client;
mod config;
mod descriptor;
mod error;
mod message;
mod network;
mod nip46;
mod policy;
mod proposal;
mod seed;
mod signer;
mod transaction;
mod user;

use self::error::Result;

// Error
pub use crate::error::FFIError;

// SmartVaults
pub use crate::abortable::AbortHandle;
pub use crate::address::{AddressIndex, GetAddress};
pub use crate::amount::Amount;
pub use crate::balance::Balance;
pub use crate::client::{SmartVaults, SyncHandler};
pub use crate::config::Config;
pub use crate::descriptor::Descriptor;
pub use crate::message::{EventHandled, Message};
pub use crate::network::Network;
pub use crate::nip46::{NostrConnectRequest, NostrConnectSession};
pub use crate::policy::{
    AbsoluteLockTime, DecayingTime, GetPolicy, Locktime, Policy, PolicyPath, PolicyPathSelector,
    PolicyPathSigner, PolicyTemplate, RecoveryTemplate, RelativeLockTime,
};
pub use crate::proposal::{
    ApprovedProposal, CompletedProposal, GetApproval, GetCompletedProposal, GetProposal, Proposal,
};
pub use crate::seed::Seed as KeychainSeed;
pub use crate::signer::{GetSharedSigner, GetSigner, SharedSigner, Signer};
pub use crate::transaction::{
    BlockTime, GetTransaction, OutPoint, Transaction, TransactionDetails, TxIn, TxOut, Utxo,
};
pub use crate::user::User;

pub fn git_hash_version() -> String {
    smartvaults_sdk::git_hash_version().to_string()
}

pub fn init_desktop_logger(base_path: String, network: Network) -> Result<()> {
    Ok(logger::init(base_path, network.into(), true)?)
}

pub fn init_mobile_logger() {
    logger::init_mobile()
}

pub fn get_keychains_list(base_path: String, network: Network) -> Result<Vec<String>> {
    Ok(smartvaults_sdk::SmartVaults::list_keychains(
        base_path,
        network.into(),
    )?)
}

// UDL
uniffi::include_scaffolding!("smartvaults_sdk");
