// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

pub use nostr_sdk_ffi::{
    EventId, Keys, Metadata, NostrConnectURI, NostrSdkError, PublicKey, Relay,
    RelayConnectionStats, RelayInformationDocument, SecretKey, Timestamp,
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
mod key_agent;
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
pub use self::error::FFIError;

// SmartVaults
pub use self::abortable::AbortHandle;
pub use self::address::{AddressIndex, GetAddress};
pub use self::amount::Amount;
pub use self::balance::Balance;
pub use self::client::{SmartVaults, SyncHandler};
pub use self::config::Config;
pub use self::descriptor::Descriptor;
pub use self::key_agent::{DeviceType, KeyAgent, Price, SignerOffering, Temperature};
pub use self::message::{EventHandled, Message};
pub use self::network::Network;
pub use self::nip46::{NostrConnectRequest, NostrConnectSession};
pub use self::policy::{
    AbsoluteLockTime, DecayingTime, GetPolicy, Locktime, Policy, PolicyPath, PolicyPathSelector,
    PolicyPathSigner, PolicyTemplate, RecoveryTemplate, RelativeLockTime,
};
pub use self::proposal::{
    ApprovedProposal, CompletedProposal, GetApproval, GetCompletedProposal, GetProposal, Period,
    Proposal,
};
pub use self::seed::Seed as KeychainSeed;
pub use self::signer::{GetSharedSigner, GetSigner, SharedSigner, Signer};
pub use self::transaction::{
    BlockTime, GetTransaction, OutPoint, Transaction, TransactionDetails, TxIn, TxOut, Utxo,
};
pub use self::user::User;

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
uniffi::include_scaffolding!("common");
