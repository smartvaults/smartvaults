use crate::users::User;
use bitcoin::Network;
use clap::{Error, Parser};
use nostr::Result;

/// The `inspect` command
#[derive(Debug, Clone, Parser)]
#[command(
    name = "inspect",
    about = "Inspect a mnemonic for bitcoin and nostr events"
)]
pub struct InspectCmd {
    /// 12 or 24 word bip32 mnemonic
    #[arg(short, long)]
    mnemonic: String,

    /// Optional passphrase
    #[arg(short, long, default_value = "")]
    passphrase: String,

    /// Optional user
    #[arg(short, long, default_value = "")]
    user: String,

    /// Optional Network, defaults to Bitcoin Testnet
    #[arg(short, long, default_value = "testnet")]
    network: String,
}

impl InspectCmd {
    pub fn run(&self, bitcoin_network: &Network) -> Result<(), Error> {
        match self.user.as_str() {
            "alice" => println!("{}", User::alice().unwrap()),
            "bob" => println!("{}", User::bob().unwrap()),
            "charlie" => println!("{}", User::charlie().unwrap()),
            "david" => println!("{}", User::david().unwrap()),
            "erika" => println!("{}", User::erika().unwrap()),
            _ => println!(
                "{}",
                User::new(&self.mnemonic, &self.passphrase, None, bitcoin_network).unwrap()
            ),
        }

        // TODO: handle result

        Ok(())
    }
}
