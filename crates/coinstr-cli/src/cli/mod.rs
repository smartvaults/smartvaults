// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::path::PathBuf;

use clap::{Parser, Subcommand};
use coinstr_sdk::core::bitcoin::{Address, XOnlyPublicKey};
use coinstr_sdk::nostr::EventId;

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
        /// Password
        #[arg(long)]
        password: Option<String>,
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
    /// Setting
    Setting {
        #[command(subcommand)]
        command: SettingCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum SettingCommand {
    /// Rename keychain
    #[command(arg_required_else_help = true)]
    Rename {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// New keychain name
        #[arg(required = true)]
        new_name: String,
    },
    /// Change keychain password
    #[command(arg_required_else_help = true)]
    ChangePassword {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
}

#[derive(Debug, Parser)]
#[command(name = "")]
pub enum Command {
    /// Inspect bitcoin and nostr keys
    Inspect,
    /// Create a spending proposal
    Spend {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// To address
        #[arg(required = true)]
        to_address: Address,
        /// Amount in sat
        #[arg(required = true)]
        amount: u64,
        /// Description
        #[arg(required = true)]
        description: String,
        /// Taget blocks
        #[clap(short, long, default_value_t = 6)]
        target_blocks: usize,
    },
    /// Create a spending proposal (send all funds)
    SpendAll {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// To address
        #[arg(required = true)]
        to_address: Address,
        /// Description
        #[arg(required = true)]
        description: String,
        /// Taget blocks
        #[clap(short, long, default_value_t = 6)]
        target_blocks: usize,
    },
    /// Approve a spending proposal
    Approve {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
    /// Finalize proposal
    Finalize {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
    /// Proof of Reserve commands
    #[command(arg_required_else_help = true)]
    Proof {
        #[command(subcommand)]
        command: ProofCommand,
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
    /// Rebroadcast all events to connected relays
    Rebroadcast,
    /// Exit
    Exit,
}

#[derive(Debug, Subcommand)]
pub enum ProofCommand {
    /// New Proof Of Reserve
    New {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// Message
        #[arg(required = true)]
        message: String,
    },
    /// Verify Proof Of Reserve
    Verify {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
}

#[derive(Debug, Subcommand)]
pub enum AddCommand {
    /// Add contact
    Contact {
        /// Public key
        #[arg(required = true)]
        public_key: XOnlyPublicKey,
    },
    /// Add policy
    Policy {
        /// Policy name
        #[arg(required = true)]
        name: String,
        /// Policy description
        #[arg(required = true)]
        description: String,
        /// Policy descriptor
        #[arg(required = true)]
        descriptor: String,
        /// Custom nostr pubkeys
        custom_pubkeys: Option<Vec<XOnlyPublicKey>>,
    },
}

#[derive(Debug, Subcommand)]
pub enum GetCommand {
    /// Get contacts list
    Contacts,
    /// Get policies list
    Policies,
    /// Get policy by id
    Policy {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// Export descriptor
        #[arg(long)]
        export: bool,
    },
    /// Get proposals list
    Proposals {
        /// Get completed proposals
        #[arg(long)]
        completed: bool,
    },
    /// Get proposal by id
    Proposal {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
    /// Get signers
    Signers,
    /// Get relays
    Relays,
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
}

#[derive(Debug, Subcommand)]
pub enum ShareCommand {
    /// Share a signer
    Signer {
        /// Signer ID
        #[arg(required = true)]
        signer_id: EventId,
        /// Public Key of the user with whom to share the signer
        #[arg(required = true)]
        public_key: XOnlyPublicKey,
    },
}

#[derive(Debug, Subcommand)]
pub enum DeleteCommand {
    /// Delete policy by id
    Policy {
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
    },
    /// Delete proposal by id
    Proposal {
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
        /// Is a completed proposals
        #[arg(long)]
        completed: bool,
    },
    /// Delete signer by id
    Signer {
        /// Signer id
        #[arg(required = true)]
        signer_id: EventId,
    },
    /// Revoke shared signer by id
    SharedSigner {
        /// Signer id
        #[arg(required = true)]
        shared_signer_id: EventId,
    },
    /// Clear cache
    Cache,
}
