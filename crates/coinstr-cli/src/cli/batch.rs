// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use super::{AddCommand, Command, SetCommand};
use clap::Parser;

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
