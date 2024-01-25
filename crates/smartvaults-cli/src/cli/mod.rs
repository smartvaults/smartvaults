// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

#![allow(clippy::large_enum_variant)]

use std::net::SocketAddr;
use std::path::PathBuf;

use clap::{Parser, Subcommand};
use smartvaults_sdk::core::bitcoin::address::NetworkUnchecked;
use smartvaults_sdk::core::bitcoin::Address;
use smartvaults_sdk::nostr::prelude::NostrConnectURI;
use smartvaults_sdk::nostr::{EventId, PublicKey, Url};
use smartvaults_sdk::protocol::v1::{BasisPoints, DeviceType, LabelData, Price, Temperature};
use smartvaults_sdk::protocol::v2::{ProposalIdentifier, SignerIdentifier, VaultIdentifier};

pub mod batch;
pub mod io;
pub mod parser;
mod types;

use self::types::{CliNetwork, CliWordCount};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about)]
pub struct Cli {
    /// Network
    #[clap(short, long, value_enum, default_value_t = CliNetwork::Bitcoin)]
    pub network: CliNetwork,
    #[command(subcommand)]
    pub command: CliCommand,
}

#[derive(Debug, Subcommand)]
pub enum CliCommand {
    /// Generate new keychain
    #[command(arg_required_else_help = true)]
    Generate {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Word count
        #[arg(value_enum, default_value_t = CliWordCount::W12)]
        word_count: CliWordCount,
        /// Passphrase
        #[arg(long)]
        passphrase: Option<String>,
    },
    /// Restore keychain
    #[command(arg_required_else_help = true)]
    Restore {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// Open keychain
    #[command(arg_required_else_help = true)]
    Open {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// Batch
    #[command(arg_required_else_help = true)]
    Batch {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Batch file
        #[arg(required = true)]
        path: PathBuf,
    },
    /// List keychains
    List,
    /// Config
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConfigCommand {
    /// View config
    View,
    /// Set config
    #[command(arg_required_else_help = true)]
    Set {
        /// Electrum server
        #[clap(long)]
        electrum_server: Option<String>,
        /// Proxy
        #[clap(long)]
        proxy: Option<SocketAddr>,
        /// Block explorer
        #[clap(long)]
        block_explorer: Option<Url>,
    },

    /// Unset
    #[command(arg_required_else_help = true)]
    Unset {
        /// Electrum server
        #[clap(long)]
        electrum_server: bool,
        /// Proxy
        #[clap(long)]
        proxy: bool,
        /// Block explorer
        #[clap(long)]
        block_explorer: bool,
    },
}

#[derive(Debug, Subcommand)]
pub enum SettingCommand {
    /// Rename keychain
    #[command(arg_required_else_help = true)]
    Rename {
        /// New keychain name
        #[arg(required = true)]
        new_name: String,
    },
    /// Change keychain password
    ChangePassword,
}

#[derive(Debug, Parser)]
#[command(name = "")]
pub enum Command {
    /// Inspect bitcoin and nostr keys
    Inspect,
    /// Create a spending proposal
    Spend {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
        /// To address
        #[arg(required = true)]
        address: Address<NetworkUnchecked>,
        /// Amount in SAT
        #[arg(required = true)]
        amount: u64,
        /// Description
        #[arg(required = true)]
        description: String,
        /// Taget blocks
        #[clap(short, long, default_value_t = 6)]
        target_blocks: u8,
    },
    /// Create a spending proposal (send all funds)
    SpendAll {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
        /// To address
        #[arg(required = true)]
        address: Address<NetworkUnchecked>,
        /// Description
        #[arg(required = true)]
        description: String,
        /// Taget blocks
        #[clap(short, long, default_value_t = 6)]
        target_blocks: u8,
    },
    /// Approve a spending proposal
    Approve {
        /// Proposal ID
        #[arg(required = true)]
        proposal_id: ProposalIdentifier,
    },
    /// Finalize proposal
    Finalize {
        /// Proposal ID
        #[arg(required = true)]
        proposal_id: ProposalIdentifier,
    },
    /// Vault commands
    #[command(arg_required_else_help = true)]
    Vault {
        #[command(subcommand)]
        command: VaultCommand,
    },
    /// Proof of Reserve commands
    #[command(arg_required_else_help = true)]
    Proof {
        #[command(subcommand)]
        command: ProofCommand,
    },
    /// Nostr Connect commands
    #[command(arg_required_else_help = true)]
    Connect {
        #[command(subcommand)]
        command: ConnectCommand,
    },
    /// Key Agent commands
    #[command(arg_required_else_help = true)]
    KeyAgent {
        #[command(subcommand)]
        command: KeyAgentCommand,
    },
    /// Add
    #[command(arg_required_else_help = true)]
    Add {
        #[command(subcommand)]
        command: AddCommand,
    },
    /// Get
    #[command(arg_required_else_help = true)]
    Get {
        #[command(subcommand)]
        command: GetCommand,
    },
    /// Set
    #[command(arg_required_else_help = true)]
    Set {
        #[command(subcommand)]
        command: SetCommand,
    },
    /// Share
    #[command(arg_required_else_help = true)]
    Share {
        #[command(subcommand)]
        command: ShareCommand,
    },
    /// Delete
    #[command(arg_required_else_help = true)]
    Delete {
        #[command(subcommand)]
        command: DeleteCommand,
    },
    /// Setting
    #[command(arg_required_else_help = true)]
    Setting {
        #[command(subcommand)]
        command: SettingCommand,
    },
    /// Rebroadcast all events to connected relays
    Rebroadcast,
    /// Exit
    Exit,
}

#[derive(Debug, Subcommand)]
pub enum VaultCommand {
    /// Add vault
    Add {
        /// Vault name
        #[arg(required = true)]
        name: String,
        /// Vault description
        #[arg(required = true)]
        description: String,
        /// Vault descriptor
        #[arg(required = true)]
        descriptor: String,
    },
    /// Invite user to vault
    Invite {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
        // User public key (hex)
        #[arg(required = true)]
        public_key: XOnlyPublicKey, // TODO: support both hex and bech32
        /// Optional message
        message: Option<String>,
    },
    /// Get vault invites
    Invites,
    /// Accept vault invite
    AcceptInvite {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
    },
    /// Update vault metadata
    Metadata {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
        // Vault name
        #[arg(short, long)]
        name: Option<String>,
        /// Vault description
        #[arg(short, long)]
        description: Option<String>,
    },
    /// Get Vault
    Get {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
        /// Export descriptor
        #[arg(long)]
        export: bool,
    },
    /// Get list of vaults
    List,
    /// Delete vault
    Delete {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
    },
}

#[derive(Debug, Subcommand)]
pub enum ProofCommand {
    /// New Proof Of Reserve
    New {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
        /// Message
        #[arg(required = true)]
        message: String,
    },
    /// Verify Proof Of Reserve
    Verify {
        /// Proposal ID
        #[arg(required = true)]
        proposal_id: ProposalIdentifier,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConnectCommand {
    /// New session
    New {
        /// Nostr Connect URI
        #[arg(required = true)]
        uri: NostrConnectURI,
    },
    /// Disconnect session
    Disconnect {
        /// App Public Key
        #[arg(required = true)]
        app_public_key: PublicKey,
    },
    /// List sessions
    Sessions,
    /// List requests
    Requests {
        /// Get approved requests
        #[arg(long)]
        approved: bool,
    },
    /// Approve request
    Approve {
        /// Request ID
        #[arg(required = true)]
        request_id: EventId,
    },
    /// Autoapprove
    Autoapprove {
        /// App Public Key
        #[arg(required = true)]
        app_public_key: PublicKey,
        /// Seconds
        #[arg(required = true)]
        seconds: u64,
    },
    /// Auto approve authorizations
    Authorizations,
    /// Revoke auto-approve
    Revoke {
        /// App Public Key
        #[arg(required = true)]
        app_public_key: PublicKey,
    },
}

#[derive(Debug, Subcommand)]
pub enum KeyAgentCommand {
    /// Create or edit signer
    Signer {
        /// Signer ID
        #[arg(required = true)]
        signer_id: SignerIdentifier,
        /// Temperature
        #[arg(required = true)]
        temperature: Temperature,
        /// Device type
        #[arg(required = true)]
        device_type: DeviceType,
        /// Response time (minutes)
        #[arg(long)]
        response_time: Option<u16>,
        /// Cost per signature (ex. 25 USD or 250000 SAT)
        #[clap(long)]
        cost_per_signature: Option<Price>,
        /// Yearly cost basis point
        #[clap(long)]
        yearly_cost_basis_points: Option<BasisPoints>,
        /// Yearly cost
        #[clap(long)]
        yearly_cost: Option<Price>,
    },
    /// List signers
    ListSigners,
}

#[derive(Debug, Subcommand)]
pub enum AddCommand {
    /// Add relay
    Relay {
        /// Url
        #[arg(required = true)]
        url: Url,
        /// Proxy
        proxy: Option<SocketAddr>,
    },
    /// Add contact
    Contact {
        /// Public key
        #[arg(required = true)]
        public_key: PublicKey,
    },
    /// Add Smart Vaults Signer
    SmartVaultsSigner {
        /// Share with contacts
        #[arg(long)]
        share_with_contacts: bool,
    },
    /// Add Coldcard Signer
    ColdcardSigner {
        /// Signer name
        #[arg(required = true)]
        name: String,
        /// Coldcard export JSON path
        #[arg(required = true)]
        path: PathBuf,
    },
}

#[derive(Debug, Subcommand)]
pub enum GetCommand {
    /// Get contacts list
    Contacts,
    /// Get proposals list
    Proposals {
        /// Get all proposals (both prending and completed)
        #[arg(long)]
        all: bool,
        /// Get only completed proposals
        #[arg(long)]
        completed: bool,
    },
    /// Get proposal by ID
    Proposal {
        /// Proposal ID
        #[arg(required = true)]
        proposal_id: ProposalIdentifier,
    },
    /// Get signers
    Signers,
    /// Get relays
    Relays,
    /// Get addresses
    Addresses {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
    },
}

#[derive(Debug, Subcommand)]
pub enum SetCommand {
    /// Set metadata
    Metadata {
        // Profile name
        #[arg(short, long)]
        name: Option<String>,
        /// Display name
        #[arg(short, long)]
        display_name: Option<String>,
        /// NIP-05
        #[arg(long)]
        nip05: Option<String>,
        /// Allow to set empty metadata
        #[arg(long)]
        empty: bool,
    },
    /// Set label
    Label {
        /// Vault ID
        #[arg(required = true)]
        vault_id: VaultIdentifier,
        /// Address, UTXO, ...
        #[arg(required = true)]
        data: LabelData,
        /// Label
        #[arg(required = true)]
        text: String,
    },
}

#[derive(Debug, Subcommand)]
pub enum ShareCommand {
    /// Share a signer
    Signer {
        /// Signer ID
        #[arg(required = true)]
        signer_id: SignerIdentifier,
        /// Public Key of the user with whom to share the signer
        #[arg(required = true)]
        public_key: PublicKey,
    },
}

#[derive(Debug, Subcommand)]
pub enum DeleteCommand {
    /// Remove relay
    Relay {
        /// Url
        #[arg(required = true)]
        url: Url,
    },
    /// Delete proposal by ID
    Proposal {
        /// Proposal ID
        #[arg(required = true)]
        proposal_id: ProposalIdentifier,
    },
    // /// Delete approval by ID
    // Approval {
    // Approval ID
    // #[arg(required = true)]
    // approval_id: EventId,
    // },
    /// Delete signer by ID
    Signer {
        /// Signer ID
        #[arg(required = true)]
        signer_id: SignerIdentifier,
    },
    // /// Revoke shared signer by ID
    // SharedSigner {
    // Signer ID
    // #[arg(required = true)]
    // shared_signer_id: SignerIdentifier,
    // },
    /// Clear cache
    Cache,
}
