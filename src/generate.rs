use clap::Parser;
use keechain_core::bitcoin::Network;
use keechain_core::types::{Seed, WordCount};
use keechain_core::util::bip::bip39::{self, Mnemonic};
use nostr_sdk::Result;

use crate::user::User;

fn generate(passphrase: Option<String>, name: Option<String>, network: Network) -> Result<User> {
	// generate a random 12 word mnemonic
	let entropy: Vec<u8> = bip39::entropy(WordCount::W12, None);
	let mnemonic = Mnemonic::from_entropy(&entropy)?;
	let seed = Seed::new(mnemonic, passphrase);
	User::new(seed, name, network)
}

/// The `generate` command
#[derive(Debug, Clone, Parser)]
#[command(name = "generate", about = "Generate a random account to work with Nostr and Bitcoin")]
pub struct GenerateCmd {
	/// The number of random accounts to generate
	#[arg(short, long, default_value_t = 1)]
	count: u8,

	/// Optional passphrase
	#[arg(short, long, default_value = "")]
	passphrase: String,
}

impl GenerateCmd {
	pub fn run(&self, bitcoin_network: Network) -> Result<()> {
		for i in 0..self.count {
			println!("\nGenerating account {} of {}", i + 1, self.count);
			let user = generate(Some(self.passphrase.clone()), None, bitcoin_network)?;
			println! {"{user}"};
		}
		Ok(())
	}
}
