use std::str::FromStr;

use bdk::miniscript::{Descriptor, DescriptorPublicKey};
use clap::Parser;
use keechain_core::bitcoin::Network;
use nostr_sdk::Result;

use crate::util;

fn balance(
	descriptor: Descriptor<DescriptorPublicKey>,
	bitcoin_endpoint: String,
	bitcoin_network: Network,
) -> Result<()> {
	println!("Balance   : {} ", util::get_balance(descriptor, bitcoin_endpoint, bitcoin_network)?);
	Ok(())
}

/// The `balance` command
#[derive(Debug, Clone, Parser)]
#[command(name = "balance", about = "Query the balance of a bitcoin descriptor")]
pub struct BalanceCmd {
	/// output descriptor
	#[arg(required = true)]
	descriptor: String,
}

impl BalanceCmd {
	pub fn run(&self, bitcoin_endpoint: String, bitcoin_network: Network) -> Result<()> {
		let desc = Descriptor::from_str(&self.descriptor)?;
		balance(desc, bitcoin_endpoint, bitcoin_network)?;
		Ok(())
	}
}
