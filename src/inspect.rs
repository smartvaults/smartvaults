use std::str::FromStr;
use clap::{Parser, Error};
use bitcoin::util::bip32;
use bip39::Mnemonic;
use crate::util;

fn inspect(mnemonic: &String) {
 
    println!("\nMnemonic : {:?} ", &mnemonic);
    
    let mnemonic = Mnemonic::parse_in_normalized(bip39::Language::English, mnemonic).unwrap();
    let path = bip32::DerivationPath::from_str("m/44'/0'/0'/0").unwrap();
    let key = (mnemonic.clone(), path);
    util::print_bitcoin(key);

    // grab the seed to use for the nostr key
    let seed: [u8; 64];
    seed = mnemonic.to_seed("".to_string());

    util::print_nostr(seed[0..32].try_into().expect("seed did not fit"));
}

/// The `inspect` command
#[derive(Debug, Clone, Parser)]
#[command(name = "inspect", about = "Inspect a mnemonic for bitcoin and nostr events")]
pub struct InspectCmd {
    /// 12 or 24 word bip32 mnemonic
    #[arg(short, long)]
    mnemonic: String,
}

impl InspectCmd {
    pub fn run(&self) -> Result<(), Error> {
     
        inspect(&self.mnemonic);

        Ok(())
    }
}
