use crate::util;
use clap::{Error, Parser};

fn balance(descriptor: &String, bitcoin_endpoint: &String, bitcoin_network: bitcoin::Network) {
    println!(
        "Balance   : {} ",
        util::get_balance(descriptor, bitcoin_endpoint, bitcoin_network).to_string()
    );
}

/// The `balance` command
#[derive(Debug, Clone, Parser)]
#[command(name = "balance", about = "Query the balance of a bitcoin descriptor")]
pub struct BalanceCmd {
    /// output descriptor
    #[arg(short, long)]
    descriptor: String,
}

impl BalanceCmd {
    pub fn run(
        &self,
        bitcoin_endpoint: &String,
        bitcoin_network: bitcoin::Network,
    ) -> Result<(), Error> {
        balance(&self.descriptor, bitcoin_endpoint, bitcoin_network);

        Ok(())
    }
}
