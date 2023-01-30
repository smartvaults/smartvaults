use clap::Parser;
use std::path::PathBuf;
use config::Config;
use bitcoin::Network;

mod balance;
mod convert;
mod generate;
// mod inspect;
// mod publish;
// mod subscribe;
mod users;
mod util;

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
    about = "Manage bitcoin and nostr together",
    version
)]
pub enum Commands {
    /// Generates random account(s)
    Generate(generate::GenerateCmd),

    // /// Subscribe to nostr events
    // Subscribe(subscribe::SubscribeCmd),

    // /// Publish a nostr event
    // Publish(publish::PublishCmd),

    // /// Inspect a mnenonic for validity and print bitcoin and nostr keys
    // Inspect(inspect::InspectCmd),

    /// Convert between hex and bech32 format keys
    Convert(convert::ConvertCmd),

    /// Find the balance for a bitcoin descriptor
    Balance(balance::BalanceCmd),
}

fn main() -> Result<(), clap::Error> {

    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("COINSTR"))
        .build()
        .unwrap();

        let mut bitcoin_network: Network = bitcoin::Network::Bitcoin;
        let bitcoin_network_str = settings.get_string("bitcoin-network").unwrap();
        if bitcoin_network_str == "testnet" {
            bitcoin_network = Network::Testnet;
        }
        let bitcoin_endpoint = settings.get_string("bitcoin-endpoint").unwrap();
        let nostr_relay = settings.get_string("nostr-relay").unwrap();

    match Commands::parse() {
        Commands::Generate(cmd) => cmd.run(),
        // Commands::Subscribe(cmd) => cmd.run(&nostr_relay),
        // Commands::Publish(cmd) => cmd.run(&nostr_relay),
        // Commands::Inspect(cmd) => cmd.run(),
        Commands::Convert(cmd) => cmd.run(),
        Commands::Balance(cmd) => cmd.run(&bitcoin_endpoint, bitcoin_network),
    }
}

// basic 2 of 3 multisig with Alice Bob and Charlie
/* thresh(2,pk(cPatMiTiN4gWsBQpKuHPY2d3Z41NWGu2xEvNumubhPADh7VHzqqV),
    pk(02476b018f75b1084e4b2bd652a747a37de9727183bcfe4113fe0b9390767e3543),
    pk(023254bcb92a82208ac8d864f3772c1576eb12dd97f1110d858cedb58251ba5043))
*/
