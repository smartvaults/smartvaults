// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::logger;

mod abortable;
mod address;
mod amount;
mod balance;
mod client;
mod config;
mod contact;
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

use self::error::Result;

// Error
pub use crate::error::FFIError;

// External
pub use coinstr_sdk::core::policy::PolicyTemplateType;
pub use coinstr_sdk::core::signer::SignerType;
pub use coinstr_sdk::core::types::WordCount;
pub use coinstr_sdk::nostr::RelayStatus;
pub use nostr_sdk_ffi::{
    EventId, Keys, Metadata, NostrConnectURI, PublicKey, Relay, RelayConnectionStats,
    RelayInformationDocument, SecretKey, Timestamp,
};

// Coinstr
pub use crate::abortable::AbortHandle;
pub use crate::address::{AddressIndex, GetAddress};
pub use crate::amount::Amount;
pub use crate::balance::Balance;
pub use crate::client::{Coinstr, SyncHandler};
pub use crate::config::Config;
pub use crate::contact::GetContact;
pub use crate::descriptor::Descriptor;
pub use crate::message::{EventHandled, Message};
pub use crate::network::Network;
pub use crate::nip46::{NostrConnectRequest, NostrConnectSession};
pub use crate::policy::{
    AbsoluteLockTime, GetPolicy, Policy, PolicyTemplate, RecoveryTemplate, RelativeLockTime,
};
pub use crate::proposal::{
    ApprovedProposal, CompletedProposal, GetApproval, GetCompletedProposal, GetProposal, Proposal,
};
pub use crate::seed::Seed as KeychainSeed;
pub use crate::signer::{GetSharedSigner, GetSigner, SharedSigner, Signer};
pub use crate::transaction::{
    BlockTime, GetTransaction, OutPoint, Transaction, TransactionDetails, TxIn, TxOut, Utxo,
};

pub fn git_hash_version() -> String {
    env!("GIT_HASH").to_string()
}

pub fn init_desktop_logger(base_path: String, network: Network) -> Result<()> {
    Ok(logger::init(base_path, network.into(), true)?)
}

pub fn init_mobile_logger() {
    logger::init_mobile()
}

pub fn get_keychains_list(base_path: String, network: Network) -> Result<Vec<String>> {
    Ok(coinstr_sdk::Coinstr::list_keychains(
        base_path,
        network.into(),
    )?)
}

// UDL
uniffi::include_scaffolding!("coinstr_sdk");
