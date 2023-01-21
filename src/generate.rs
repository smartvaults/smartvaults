
use bdk::keys::SinglePriv;
use clap::Parser;
use nostr_rust::bech32::{to_bech32, ToBech32Kind};
use nostr_rust::keys::*;
// use bdk::bitcoin::secp256k1::SecretKey;
use rand::rngs::OsRng;

extern crate serde_json;
// use bitcoin::network::constants::Network;
use bdk::{keys::{
    bip39::{Language, Mnemonic, WordCount},
    DerivableKey, ExtendedKey, GeneratableKey, GeneratedKey,
}};

use bdk::miniscript;
//use secp256k1::SecretKey;
use secp256k1::{PublicKey, SecretKey, Secp256k1};
// use bdk::bitcoin::{PrivateKey};
use bdk::bitcoin::{Network};
use bitcoin::util::key::PrivateKey;


fn dump_keys(_sk_str: String) {


    // let (secret_key_1, _) = get_random_secret_key();
    // let (secret_str_1, _) = get_str_keys_from_secret(&secret_key_1);

    println!("\nnostr generation: ");

    let secp = Secp256k1::new();
    let (secret_key, public_key) = secp.generate_keypair(&mut OsRng);

    let secret_key_str =  secret_key.display_secret().to_string();

    println!("Secret Key (HEX): {:?} ", secret_key_str) ;
    println!("Public Key (HEX): {:?} ", public_key.to_string());

    let bech32_pub = to_bech32(ToBech32Kind::PublicKey, &public_key.to_string());
    let bech32_prv = to_bech32(ToBech32Kind::SecretKey, &secret_key_str);

    println!("Public Key (bech32): {:?} ", bech32_pub.unwrap());
    println!("Secret Key (bech32): {:?} ", bech32_prv.unwrap());

    println!("\nBDK generation: ");
    
    let prv: PrivateKey = PrivateKey::from_slice(&secret_key.secret_bytes(), Network::Testnet).unwrap();
    // let prv = PrivateKey::new_uncompressed(secret_key, Network::Testnet);

    let single_prv : bdk::keys::SinglePriv = SinglePriv {
        origin: None,
        key: prv
    };

    println!("Bitcoin Secret Key: {:?} ", single_prv.key.to_string());


    // Generate fresh mnemonic
    // let mnemonic: GeneratedKey<_, miniscript::Segwitv0> =
    //     Mnemonic::generate((WordCount::Words12, Language::English)).unwrap();

    // // Convert mnemonic to string
    // let mnemonic_words = mnemonic.to_string();
    // println!("Mnemonic: {:?} ", &mnemonic_words);

    // // Parse a mnemonic
    // let mnemonic = Mnemonic::parse(&mnemonic_words).unwrap();

    // // Generate the extended key
    // let xkey: ExtendedKey = mnemonic.into_extended_key().unwrap();

    // // Get xprv from the extended key
    // let xprv = xkey.into_xprv(network).unwrap();
   
    //
}

/// The `generate` command
#[derive(Debug, Clone, Parser)]
#[command(name = "generate", about = "Generate a random account")]
pub struct GenerateCmd {
    
    /// The number of random accounts to generate
    #[arg(short, long, default_value_t = 1)]
    count: u8,
}

use clap::Error;
impl GenerateCmd {
    /// Run the command
    pub fn run(&self) -> Result<(), Error>  {
        for i in 0..self.count {
           
            println!("Generating random key: {} of {}", i, self.count);
            dump_keys("".to_string());
            println!();
        }

        Ok(())
    }
}
