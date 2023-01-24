use crate::util;
use bdk::keys::bip39::{Language::English, Mnemonic};
use clap::{Error, Parser};
use nostr::Result;

fn inspect(mnemonic: &String, passphrase: &String) -> Result<()> {
    let mnemonic = Mnemonic::parse_in_normalized(English, mnemonic).unwrap();
    println!("\nMnemonic : {:?} ", &mnemonic);

    println!("\nMnemonic   : \"{}\" ", &mnemonic.to_string());
    println!("Passphrase : \"{}\" ", &passphrase.to_string());

    util::print_nostr(&mnemonic, passphrase)?;
    util::print_bitcoin(&mnemonic, passphrase)?;

    Ok(())
}

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
}

impl InspectCmd {
    pub fn run(&self) -> Result<(), Error> {
        
        // TODO: handle result
        inspect(&self.mnemonic, &self.passphrase);

        Ok(())        
    }
}
