
use std::str::FromStr;
use bitcoin::XOnlyPublicKey;
use clap::{Parser, Error};
use nostr::prelude::ToBech32;
use nostr::key::Keys;
use nostr::key::FromSkStr;

/// The `convert` command
#[derive(Debug, Clone, Parser)]
#[command(name = "convert", about = "Convert one type of key to another format")]
pub struct ConvertCmd {
    /// 32-byte Hex key
    #[arg(short, long)]
    input_key: String,

    /// is the provided key a private key?
    #[arg(short, long)]
    secret: bool,

    /// if bech32 is provided, convert to hex
    #[arg(short, long, default_value_t = false)]
    to_hex: bool,
}

impl ConvertCmd {
    pub fn run(&self) -> Result<(), Error> {
     
        println!("\nNostr Configuration");
       
        let key : Keys;
        if self.secret {
            key = Keys::from_sk_str(&self.input_key).unwrap();
            let secret_key_str = key.secret_key().unwrap().display_secret().to_string();
            assert_eq!(self.input_key, secret_key_str);

            let bech32_prv = key.secret_key().unwrap().to_bech32().unwrap();
            println!("  Secret Key (HEX)    : {} ", secret_key_str);
            println!("  Secret Key (bech32) : {} ", bech32_prv);

            let public_key = key.public_key();
            println!("  Public Key (HEX)    : {} ", public_key);
            let bech32_pub = public_key.to_bech32().unwrap();
            println!("  Public Key (bech32) : {} ", bech32_pub);

        } else {
            println!("Only converting public key");
           
            let public_key = XOnlyPublicKey::from_str(&self.input_key).unwrap();
            println!("  Public Key (HEX)    : {} ", public_key);

            let bech32_pub = public_key.to_bech32().unwrap();
            println!("  Public Key (bech32) : {} ", bech32_pub);
        }
                
        Ok(())
    }
}
