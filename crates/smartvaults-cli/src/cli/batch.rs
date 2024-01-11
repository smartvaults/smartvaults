// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use clap::Parser;

use super::{AddCommand, Command, SetCommand};

#[derive(Debug, Parser)]
#[command(name = "")]
pub enum BatchCommand {
    /// Add
    #[command(arg_required_else_help = true)]
    Add {
        #[command(subcommand)]
        command: AddCommand,
    },
    /// Set
    #[command(arg_required_else_help = true)]
    Set {
        #[command(subcommand)]
        command: SetCommand,
    },
}

impl From<BatchCommand> for Command {
    fn from(cmd: BatchCommand) -> Self {
        match cmd {
            BatchCommand::Add { command } => Self::Add { command },
            BatchCommand::Set { command } => Self::Set { command },
        }
    }
}
