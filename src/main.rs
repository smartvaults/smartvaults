use std::{path::PathBuf, str::FromStr};

use clap::{Args, Parser, Subcommand};
use config::Config;
use keechain_core::bitcoin::Network;
use nostr_sdk::Result;

mod command;
mod orchestration;
mod policy;
mod user;
mod util;

const DEFAULT_TESTNET_ENDPOINT: &str = "ssl://blockstream.info:993"; // or ssl://electrum.blockstream.info:60002
const DEFAULT_BITCOIN_ENDPOINT: &str = "ssl://blockstream.info:700"; // or ssl://electrum.blockstream.info:50002
#[allow(unused)]
const DEFAULT_RELAY: &str = "wss://relay.rip";

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
	Generate(command::generate::GenerateCmd),

	/// Subscribe to nostr events
	Subscribe(command::subscribe::SubscribeCmd),

	/// Publish a nostr event
	Publish(command::publish::PublishCmd),

	/// Inspect a mnenonic for validity and print bitcoin and nostr keys
	Inspect(command::inspect::InspectCmd),

	/// Convert between hex and bech32 format keys
	Convert(command::convert::ConvertCmd),

	/// Find the balance for a bitcoin descriptor
	Balance(command::balance::BalanceCmd),

	/// Save policy
	SavePolicy(command::save_policy::SavePolicyCmd),

	/// Get data about events and users
	#[command(arg_required_else_help = true)]
	Get(Box<GetArgs>),
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct GetArgs {
	#[command(subcommand)]
	command: Option<GetCommands>,

	#[command(flatten)]
	events: command::get_events::GetEventsCmd,
}

#[derive(Debug, Subcommand)]
enum GetCommands {
	Event(command::get_event::GetEventCmd),
	Events(command::get_events::GetEventsCmd),
	Users(command::get_users::GetUsersCmd),
	User(command::get_user::GetUserCmd),
	Contacts(command::get_contacts::GetContactsCmd),
	Policies(command::get_policies::GetPoliciesCmd),
}

fn main() -> Result<()> {
	env_logger::init();

	let settings = Config::builder()
		.add_source(config::File::with_name("config"))
		.add_source(config::Environment::with_prefix("COINSTR"))
		.build()?;

	let bitcoin_network_str = settings.get_string("bitcoin-network")?;
	let bitcoin_network: Network = Network::from_str(&bitcoin_network_str)?;
	let bitcoin_endpoint = settings.get_string("bitcoin-endpoint")?;
	let nostr_relay = settings.get_string("nostr-relay")?;

	match Commands::parse() {
		Commands::Generate(cmd) => cmd.run(bitcoin_network),
		Commands::Subscribe(cmd) => cmd.run(&nostr_relay),
		Commands::Publish(cmd) => cmd.run(nostr_relay),
		Commands::Inspect(cmd) => cmd.run(bitcoin_network),
		Commands::Convert(cmd) => cmd.run(),
		Commands::Balance(cmd) => cmd.run(bitcoin_endpoint, bitcoin_network),
		Commands::SavePolicy(cmd) => cmd.run(nostr_relay),
		Commands::Get(cmd) => match cmd.command.unwrap() {
			GetCommands::Event(get_cmd) => get_cmd.run(nostr_relay),
			GetCommands::Events(get_cmd) => get_cmd.run(nostr_relay),
			GetCommands::Users(get_cmd) => get_cmd.run(),
			GetCommands::User(get_cmd) => get_cmd.run(),
			GetCommands::Contacts(get_cmd) => get_cmd.run(nostr_relay),
			GetCommands::Policies(get_cmd) => get_cmd.run(nostr_relay),
		},
	}
}

// basic 2 of 3 multisig with Alice Bob and Charlie
/* thresh(2,pk(cPatMiTiN4gWsBQpKuHPY2d3Z41NWGu2xEvNumubhPADh7VHzqqV),
	pk(02476b018f75b1084e4b2bd652a747a37de9727183bcfe4113fe0b9390767e3543),
	pk(023254bcb92a82208ac8d864f3772c1576eb12dd97f1110d858cedb58251ba5043))
*/
