use std::str::FromStr;

use clap::Parser;
use keechain_core::{bip39::Mnemonic, bitcoin::Network, types::Seed};
use nostr_sdk::Result;

use crate::user::User;

/// The `inspect` command
#[derive(Debug, Clone, Parser)]
#[command(name = "inspect", about = "Inspect a mnemonic for bitcoin and nostr events")]
pub struct InspectCmd {
	/// 12 or 24 word bip32 mnemonic
	#[arg(short, long)]
	mnemonic: String,

	/// Optional passphrase
	#[arg(short, long)]
	passphrase: Option<String>,

	/// Optional user
	#[arg(short, long, default_value = "")]
	user: String,

	/// Optional Network, defaults to Bitcoin Testnet
	#[arg(short, long, default_value = "mainnet")]
	network: String,
}

impl InspectCmd {
	pub fn run(&self, bitcoin_network: Network) -> Result<()> {
		match self.user.as_str() {
			"alice" => println!("{}", User::alice()?),
			"bob" => println!("{}", User::bob()?),
			"charlie" => println!("{}", User::charlie()?),
			"david" => println!("{}", User::david()?),
			"erika" => println!("{}", User::erika()?),
			_ => {
				let mnemonic = Mnemonic::from_str(&self.mnemonic)?;
				println!(
					"{}",
					User::new(Seed::new(mnemonic, self.passphrase.clone()), None, bitcoin_network)?
				)
			},
		}
		Ok(())
	}
}
