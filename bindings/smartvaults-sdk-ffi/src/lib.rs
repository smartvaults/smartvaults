// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

#![allow(clippy::new_without_default)]

nostr_sdk_ffi::uniffi_reexport_scaffolding!();

use uniffi::Object;

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

pub use self::abortable::AbortHandle;
pub use self::address::{AddressIndex, GetAddress};
pub use self::amount::Amount;
pub use self::balance::Balance;
pub use self::client::{SmartVaults, SyncHandler};
pub use self::config::Config;
pub use self::descriptor::Descriptor;
use self::error::Result;
pub use self::error::SmartVaultsError;
pub use self::key_agent::{DeviceType, KeyAgent, Price, SignerOffering, Temperature};
pub use self::message::{EventHandled, Message};
pub use self::network::Network;
pub use self::nip46::{NostrConnectRequest, NostrConnectSession};
pub use self::policy::{
    AbsoluteLockTime, DecayingTime, GetPolicy, Locktime, Policy, PolicyPath, PolicyPathSelector,
    PolicyPathSigner, PolicyTemplate, PolicyTemplateType, RecoveryTemplate, RelativeLockTime,
};
pub use self::proposal::{
    ApprovedProposal, CompletedProposal, GetApproval, GetCompletedProposal, GetProposal, Period,
    Proposal,
};
pub use self::seed::{Seed, WordCount};
pub use self::signer::{GetSharedSigner, GetSigner, SharedSigner, Signer, SignerType};
pub use self::transaction::{
    BlockTime, GetTransaction, OutPoint, Transaction, TransactionDetails, TxIn, TxOut, Utxo,
};

#[derive(Object)]
pub struct SmartVaultsLibrary;

#[uniffi::export]
impl SmartVaultsLibrary {
    #[uniffi::constructor]
    pub fn new() -> Self {
        Self
    }

    pub fn git_hash_version(&self) -> String {
        smartvaults_sdk::git_hash_version().to_string()
    }
}

#[uniffi::export]
pub fn init_desktop_logger(base_path: String, network: Network) -> Result<()> {
    Ok(smartvaults_sdk::logger::init(
        base_path,
        network.into(),
        true,
    )?)
}

#[uniffi::export]
pub fn init_mobile_logger() {
    smartvaults_sdk::logger::init_mobile()
}

#[uniffi::export]
pub fn get_keychains_list(base_path: String, network: Network) -> Result<Vec<String>> {
    Ok(smartvaults_sdk::SmartVaults::list_keychains(
        base_path,
        network.into(),
    )?)
}

uniffi::setup_scaffolding!("smartvaults_sdk");
