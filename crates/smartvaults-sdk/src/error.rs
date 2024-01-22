// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::database::DatabaseError;
use nostr_sdk::SQLiteError;
use smartvaults_protocol::v1::util::EncryptionError;
use smartvaults_protocol::v1::SmartVaultsEventBuilderError;
use thiserror::Error;

use crate::manager::{Error as ManagerError, WalletError};
use crate::util;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Keechain(#[from] smartvaults_core::types::keechain::Error),
    #[error(transparent)]
    Keychain(#[from] smartvaults_core::types::keychain::Error),
    #[error(transparent)]
    Dir(#[from] util::dir::Error),
    #[error(transparent)]
    JSON(#[from] serde_json::Error),
    #[error(transparent)]
    Electrum(#[from] bdk_electrum::electrum_client::Error),
    #[error(transparent)]
    Url(#[from] nostr_sdk::types::url::ParseError),
    #[error(transparent)]
    Client(#[from] nostr_sdk::client::Error),
    #[error(transparent)]
    RelayPool(#[from] nostr_sdk::relay::pool::Error),
    #[error(transparent)]
    NostrDatabase(#[from] DatabaseError),
    #[error(transparent)]
    NostrDatabaseSQLite(#[from] SQLiteError),
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
    #[error(transparent)]
    EventId(#[from] nostr_sdk::event::id::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr_sdk::event::builder::Error),
    #[error(transparent)]
    SmartVaultsEventBuilder(#[from] SmartVaultsEventBuilderError),
    #[error(transparent)]
    Relay(#[from] nostr_sdk::relay::Error),
    #[error(transparent)]
    Policy(#[from] smartvaults_core::policy::Error),
    #[error(transparent)]
    Proposal(#[from] smartvaults_core::proposal::Error),
    #[error(transparent)]
    Secp256k1(#[from] smartvaults_core::bitcoin::secp256k1::Error),
    #[error(transparent)]
    Address(#[from] smartvaults_core::bitcoin::address::Error),
    #[error(transparent)]
    Encryption(#[from] EncryptionError),
    #[error(transparent)]
    NIP04(#[from] nostr_sdk::nips::nip04::Error),
    #[error(transparent)]
    NIP06(#[from] nostr_sdk::nips::nip06::Error),
    #[error(transparent)]
    NIP46(#[from] nostr_sdk::nips::nip46::Error),
    #[error(transparent)]
    BIP32(#[from] smartvaults_core::bitcoin::bip32::Error),
    #[error(transparent)]
    Signer(#[from] smartvaults_core::signer::Error),
    #[error(transparent)]
    Manager(#[from] ManagerError),
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Config(#[from] crate::config::Error),
    #[error(transparent)]
    Store(#[from] smartvaults_sdk_sqlite::Error),
    #[error(transparent)]
    Label(#[from] smartvaults_protocol::v1::label::Error),
    #[error(transparent)]
    KeyAgentVerified(#[from] smartvaults_protocol::v1::key_agent::verified::Error),
    #[error("password not match")]
    PasswordNotMatch,
    #[error("not enough public keys")]
    NotEnoughPublicKeys,
    #[error("shared keys not found")]
    SharedKeysNotFound,
    #[error("policy not found")]
    PolicyNotFound,
    #[error("proposal not found")]
    ProposalNotFound,
    #[error("unexpected proposal")]
    UnexpectedProposal,
    #[error("approved proposal/s not found")]
    ApprovedProposalNotFound,
    #[error("signer not found")]
    SignerNotFound,
    #[error("signer ID not found")]
    SignerIdNotFound,
    #[error("public key not found")]
    PublicKeyNotFound,
    #[error("signer already shared")]
    SignerAlreadyShared,
    #[error("signer descriptor already exists")]
    SignerDescriptorAlreadyExists,
    #[error("nostr connect request already approved")]
    NostrConnectRequestAlreadyApproved,
    #[error("impossible to generate nostr connect response")]
    CantGenerateNostrConnectResponse,
    #[error("invalid fee rate")]
    InvalidFeeRate,
    #[error("impossible to delete a not owned event")]
    TryingToDeleteNotOwnedEvent,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Generic(String),
}
