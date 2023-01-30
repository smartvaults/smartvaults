use crate::util;
use bdk::keys::bip39::{Language::English, Mnemonic};
use clap::{Error, Parser};
use nostr::Result;
use crate::users::User;

fn inspect(mnemonic: &String, passphrase: &String) -> Result<()> {
   
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

    /// Optional user
    #[arg(short, long, default_value = "")]
    user: String,
}

impl InspectCmd {
    pub fn run(&self) -> Result<(), Error> {

        match self.user.as_str() {
            "alice" => println!("{}", User::alice()),
            "bob" => println!("{}", User::bob()),
            "charlie" => User::charlie(),
            "david" => User::david(),
            "erika" => User::erika(),
            _ => println!("{}", User {
                mnemonic: self.mnemonic, 
                passphrase: self.passphrase,
            }),
        }
        
        // TODO: handle result
        

        Ok(())        
    }
}
