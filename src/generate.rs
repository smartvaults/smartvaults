use bdk::keys::{bip39::WordCount, GeneratableKey, GeneratedKey};
use bdk::miniscript;
use bip39::{Language, Mnemonic};
use clap::{Error, Parser};

use crate::util;

fn generate() {
    // generate a random 12 word mnemonic
    let mnemonic: GeneratedKey<_, miniscript::Segwitv0> =
        Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();

    util::print_keys(mnemonic);
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
}

impl GenerateCmd {
    pub fn run(&self) -> Result<(), Error> {
        for i in 0..self.count {
            println!("\nGenerating account {} of {}", i + 1, self.count);
            generate();
            println!();
        }

        Ok(())
    }
}
