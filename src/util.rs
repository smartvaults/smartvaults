use bdk::keys::GeneratedKey;
use bdk::miniscript;
use bdk::wallet::Wallet;
use bip39::Mnemonic;
use bitcoin::util::bip32::DerivationPath;
use nostr_rust::bech32::{to_bech32, ToBech32Kind};
use secp256k1::Secp256k1;
use std::str::FromStr;

pub fn print_nostr (seed: [u8; 32]) {
    let secp = Secp256k1::new();

    // mnemonic creates 64-bytes, but we only use the first 32
    let secret_key = secp256k1::SecretKey::from_slice(&seed).unwrap();
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

    let secret_key_str = secret_key.display_secret().to_string();

    println!("\nNostr Configuration");
    println!("  Secret Key (HEX)    : {} ", secret_key_str);
    println!("  Public Key (HEX)    : {} ", public_key.to_string());

    let bech32_pub = to_bech32(ToBech32Kind::PublicKey, &public_key.to_string());
    let bech32_prv = to_bech32(ToBech32Kind::SecretKey, &secret_key_str);

    println!("  Public Key (bech32) : {} ", bech32_pub.unwrap());
    println!("  Secret Key (bech32) : {} ", bech32_prv.unwrap());
}

pub fn print_bitcoin (key : (Mnemonic, DerivationPath)) {
    
    let (desc, _, _) = bdk::descriptor!(wpkh(key)).unwrap();
    println!("\nBitcoin Configuration");
    println!("  Output Descriptor   : {}", desc.to_string());

    let db = bdk::database::memory::MemoryDatabase::new();
    let wallet = Wallet::new(desc, None, bitcoin::Network::Bitcoin, db);
    let address = wallet
        .as_ref()
        .unwrap()
        .get_address(bdk::wallet::AddressIndex::New)
        .unwrap();
    println!("  First Address       : {} ", address.to_string());

    let address = wallet
        .unwrap()
        .get_address(bdk::wallet::AddressIndex::New)
        .unwrap();
    println!("  Second Address      : {} ", address.to_string());
}

pub fn print_keys (mnemonic: GeneratedKey<Mnemonic, miniscript::Segwitv0>) {
    println!("\nMnemonic : \"{}\" ", &mnemonic.to_string());

    let path = DerivationPath::from_str("m/44'/0'/0'/0").unwrap();
    let key = (mnemonic.clone().into_key(), path);

    print_bitcoin(key);   

    // grab the seed to use for the nostr key
    let seed: [u8; 64];
    seed = mnemonic.to_seed("".to_string());

    print_nostr(seed[0..32].try_into().expect("seed did not fit"));
}