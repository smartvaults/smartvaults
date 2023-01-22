use clap::Parser;
use std::path::PathBuf;
extern crate serde_json;

mod generate;
mod publish;
mod subscribe;
mod users;
mod inspect;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional name to operate on
    #[arg(short, long, value_name = "name")]
    name: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Debug, Parser)]
#[command(
    name = "coinstr",
    author = "3yekn",
    about = "Manage bitcoin with nostr",
    version
)]
pub enum Commands {
    /// Generates random account(s)
    Generate(generate::GenerateCmd),

    /// Subscribe to nostr events
    Subscribe(subscribe::SubscribeCmd),

    /// Publish a nostr event
    Publish(publish::PublishCmd),

    /// Inspect a Mnenonic for validity and print bitcoin and nostr keys
    Inspect(inspect::InspectCmd),
}

fn main() -> Result<(), clap::Error> {

    match Commands::parse() {
        Commands::Generate(cmd) => cmd.run(),
        Commands::Subscribe(cmd) => cmd.run(),
        Commands::Publish(cmd) => cmd.run(),
        Commands::Inspect(cmd) => cmd.run(),
    }
}

// basic 2 of 3 multisig with Alice Bob and Charlie
/* thresh(2,pk(cPatMiTiN4gWsBQpKuHPY2d3Z41NWGu2xEvNumubhPADh7VHzqqV),
    pk(02476b018f75b1084e4b2bd652a747a37de9727183bcfe4113fe0b9390767e3543),
    pk(023254bcb92a82208ac8d864f3772c1576eb12dd97f1110d858cedb58251ba5043))
*/
