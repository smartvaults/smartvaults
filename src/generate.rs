use crate::user::users::User;
use bitcoin::Network;
use clap::Parser;
use nostr::nips::nip06::GenerateMnemonic;
use nostr::Keys;
// use std::error::Error;
use anyhow::Result;
use anyhow::bail;

fn generate(
    passphrase: &String,
    name: Option<String>,
    network: &bitcoin::Network,
) -> Result<User> {
    // generate a random 12 word mnemonic
    // let mnemonic = Keys::generate_mnemonic(12)?.to_string();
    // User::new(&mnemonic, passphrase, name, network)
    // Ok(())
    bail!("broken for now")
}

/// The `generate` command
#[derive(Debug, Clone, Parser)]
#[command(
    name = "generate",
    about = "Generate a random account to work with Nostr and Bitcoin"
)]
pub struct GenerateCmd {
    /// The number of random accounts to generate
    #[arg(short, long, default_value_t = 1)]
    count: u8,

    /// Optional passphrase
    #[arg(short, long, default_value = "")]
    passphrase: String,
}

impl GenerateCmd {
    pub fn run(&self, bitcoin_network: &Network) -> Result<(), clap::Error> {
        for i in 0..self.count {
            println!("\nGenerating account {} of {}", i + 1, self.count);
            let user = generate(&self.passphrase, None, bitcoin_network);
            if user.is_ok() {
                println! {"{}", user.unwrap()};
            }
        }
        Ok(())
    }
}
