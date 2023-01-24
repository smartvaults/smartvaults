use clap::{Error, Parser};
use nostr::nips::nip06::GenerateMnemonic;
use nostr::{Keys, Result};

use crate::util;

fn generate(passphrase: &String) -> Result<()> {
    // generate a random 12 word mnemonic
    let keys = Keys::generate_mnemonic(12)?;

    util::print_keys(&keys, passphrase)?;
    Ok(())
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
    pub fn run(&self) -> Result<(), Error> {
        for i in 0..self.count {
            println!("\nGenerating account {} of {}", i + 1, self.count);
           generate(&self.passphrase);
           println!();
        }

        Ok(())
    }
}
