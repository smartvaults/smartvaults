use std::str::FromStr;

use clap::{Parser, Error};
use nostr_rust::bech32::{to_bech32, ToBech32Kind};
// use bdk::keys::bip39::Mnemonic;
use secp256k1::Secp256k1;
use bdk::wallet::Wallet;
use bitcoin::util::bip32;
use bip39::Mnemonic;

fn inspect(mnemonic: &String, passphrase: &String) {
 
    println!("Mnemonic: {:?} ", &mnemonic);

    let mnemonic = Mnemonic::parse_in_normalized(bip39::Language::English, mnemonic).unwrap();
    let seed = mnemonic.to_seed_normalized(passphrase);
    println!("seed: {:?}", seed);

    let path = bip32::DerivationPath::from_str("m/44'/0'/0'/0").unwrap();

    let key = (mnemonic, path);
    let (desc, _keys, _networks) = bdk::descriptor!(wpkh(key)).unwrap();
    println!("Bitcoin Inspection : ");
    println!("Bitcoin Output Descriptor: {}", desc.to_string());

    let db = bdk::database::memory::MemoryDatabase::new();
    let wallet = Wallet::new(desc, None, bitcoin::Network::Bitcoin, db);
    let address = wallet
        .unwrap()
        .get_address(bdk::wallet::AddressIndex::New)
        .unwrap();
    println!("First Address : {} ", address.to_string());

    println!("\nNostr Inspection : ");
    let secp = Secp256k1::new();

    let secret_key = secp256k1::SecretKey::from_slice(&seed[0..32]).unwrap();
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    let secret_key_str = secret_key.display_secret().to_string();

    println!("Nostr Secret Key (HEX): {:?} ", secret_key_str);
    println!("Nostr Public Key (HEX): {:?} ", public_key.to_string());

    let bech32_pub = to_bech32(ToBech32Kind::PublicKey, &public_key.to_string());
    let bech32_prv = to_bech32(ToBech32Kind::SecretKey, &secret_key_str);

    println!("Nostr Public Key (bech32): {:?} ", bech32_pub.unwrap());
    println!("Nostr Secret Key (bech32): {:?} ", bech32_prv.unwrap());

}

/// The `inspect` command
#[derive(Debug, Clone, Parser)]
#[command(name = "inspect", about = "Inspect a mnemonic for bitcoin and nostr events")]
pub struct InspectCmd {
    /// 12 or 24 word bip32 mnemonic
    #[arg(short, long)]
    mnemonic: String,

    /// Optional 
    #[arg(short, long, default_value = "")]
    passphrase: String,
}

impl InspectCmd {
    pub fn run(&self) -> Result<(), Error> {
     
        inspect(&self.mnemonic, &self.passphrase);

        Ok(())
    }
}
