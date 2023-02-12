use bitcoin::Network;
use clap::{Args, Parser, Subcommand};
use config::Config;
use std::path::PathBuf;

mod balance;
mod convert;
mod generate;
mod get_events;
mod get_user;
mod get_users;
mod inspect;
mod publish;
// mod subscribe;
mod user;
mod util;
mod policy;

#[derive(Parser)]
#[command(name = "coinstr")]
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
    about = "Using nostr to coordinate Bitcoin spending policy signatures and multi-custody",
    version
)]
pub enum Commands {
    /// Generates random account(s)
    Generate(generate::GenerateCmd),

    /// Subscribe to nostr events
    // Subscribe(subscribe::SubscribeCmd),

    /// Publish a nostr event
    Publish(publish::PublishCmd),

    /// Inspect a mnenonic for validity and print bitcoin and nostr keys
    Inspect(inspect::InspectCmd),

    /// Convert between hex and bech32 format keys
    Convert(convert::ConvertCmd),

    /// Find the balance for a bitcoin descriptor
    Balance(balance::BalanceCmd),

    /// Get things
    #[command(arg_required_else_help = true)]
    Get(GetArgs),
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct GetArgs {
    #[command(subcommand)]
    command: Option<GetCommands>,

    #[command(flatten)]
    events: get_events::GetEventsCmd,
}

#[derive(Debug, Subcommand)]
enum GetCommands {
    Events(get_events::GetEventsCmd),
    Users(get_users::GetUsersCmd),
    User(get_user::GetUserCmd),
}

fn main() -> Result<(), clap::Error> {
    let settings = Config::builder()
        .add_source(config::File::with_name("config"))
        .add_source(config::Environment::with_prefix("COINSTR"))
        .build()
        .unwrap();

    let mut bitcoin_network: Network = bitcoin::network::constants::Network::Bitcoin;
    let bitcoin_network_str = settings.get_string("bitcoin-network").unwrap();
    if bitcoin_network_str == "testnet" {
        bitcoin_network = bitcoin::network::constants::Network::Testnet;
    } 
    let bitcoin_endpoint = settings.get_string("bitcoin-endpoint").unwrap();
    let nostr_relay = settings.get_string("nostr-relay").unwrap();

    match Commands::parse() {
        Commands::Generate(cmd) => cmd.run(&bitcoin_network),
        // Commands::Subscribe(cmd) => cmd.run(&nostr_relay),
        Commands::Publish(cmd) => cmd.run(&nostr_relay),
        Commands::Inspect(cmd) => cmd.run(&bitcoin_network),
        Commands::Convert(cmd) => cmd.run(),
        Commands::Balance(cmd) => cmd.run(&bitcoin_endpoint, bitcoin_network),
        Commands::Get(cmd) => {
            let get_cmd = cmd.command.unwrap();
            match get_cmd {
                GetCommands::Events(get_cmd) => get_cmd.run(&nostr_relay),
                GetCommands::Users(get_cmd) => get_cmd.run(),
                GetCommands::User(get_cmd) => get_cmd.run(),
            }
        }
    }
}

// basic 2 of 3 multisig with Alice Bob and Charlie
/* thresh(2,pk(cPatMiTiN4gWsBQpKuHPY2d3Z41NWGu2xEvNumubhPADh7VHzqqV),
    pk(02476b018f75b1084e4b2bd652a747a37de9727183bcfe4113fe0b9390767e3543),
    pk(023254bcb92a82208ac8d864f3772c1576eb12dd97f1110d858cedb58251ba5043))
*/
