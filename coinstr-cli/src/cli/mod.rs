use clap::{Parser, Subcommand};
use coinstr_core::bitcoin::Address;
use coinstr_core::nostr_sdk::EventId;

pub mod io;
mod types;

use self::types::{CliNetwork, CliWordCount};

#[derive(Debug, Parser)]
#[clap(author, version, about, long_about = None)]
pub struct Cli {
    /// Network
    #[clap(short, long, value_enum, default_value_t = CliNetwork::Bitcoin)]
    pub network: CliNetwork,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Generate new keychain
    #[command(arg_required_else_help = true)]
    Generate {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Word count
        #[arg(value_enum, default_value_t = CliWordCount::W12)]
        word_count: CliWordCount,
    },
    /// Restore keychain
    #[command(arg_required_else_help = true)]
    Restore {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// List keychains
    List,
    /// Inspect bitcoin and nostr keys
    #[command(arg_required_else_help = true)]
    Inspect {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// Save policy
    SavePolicy {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Policy name
        #[arg(required = true)]
        policy_name: String,
        /// Policy description
        #[arg(required = true)]
        policy_description: String,
        /// Policy descriptor
        #[arg(required = true)]
        policy_descriptor: String,
    },
    /// Create a spending proposal
    Spend {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
        /// Memo
        #[arg(required = true)]
        memo: String,
        /// To address
        #[arg(required = true)]
        to_address: Address,
        /// Amount in sats
        #[arg(required = true)]
        amount: u64,
    },
    /// Approve a spending proposal
    Approve {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
    },
    /// Get data about events and users
    #[command(arg_required_else_help = true)]
    Get {
        #[command(subcommand)]
        command: GetCommand,
    },
    /// Setting
    Setting {
        #[command(subcommand)]
        command: SettingCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum GetCommand {
    /// Get contacts list from nostr
    Contacts {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// Get policies list from nostr
    Policies {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// Get policy by id
    Policy {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Policy id
        #[arg(required = true)]
        policy_id: EventId,
    },
    /// Get proposals list from nostr
    Proposals {
        /// Keychain name
        #[arg(required = true)]
        name: String,
    },
    /// Get proposal by id
    Proposal {
        /// Keychain name
        #[arg(required = true)]
        name: String,
        /// Proposal id
        #[arg(required = true)]
        proposal_id: EventId,
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
