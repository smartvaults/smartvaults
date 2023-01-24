use bdk::keys::bip39::Mnemonic;
use bdk::keys::DerivableKey;
use bdk::wallet::Wallet;
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey};
use bitcoin::Network;
use nostr::nips::nip19::ToBech32;
use nostr::{prelude::FromMnemonic, Keys, Result};
use std::str::FromStr;

pub fn print_nostr(mnemonic: &Mnemonic, passphrase: &String) -> Result<()> {

    let keys = Keys::from_mnemonic(mnemonic.to_string(), Some(passphrase.to_string())).unwrap();

    println!("\nNostr Configuration");
    println!(
        "  Secret Key (HEX)    : {} ",
        keys.secret_key()?.display_secret().to_string()
    );
    println!(
        "  Secret Key (bech32) : {} ",
        keys.secret_key()?.to_bech32()?.to_string()
    );
    println!("  Public Key (HEX)    : {} ", keys.public_key().to_string());
    println!(
        "  Public Key (bech32) : {} ",
        keys.public_key().to_bech32()?.to_string()
    );
    Ok(())
}

pub fn print_bitcoin(mnemonic: &Mnemonic, passphrase: &String) -> Result<()> {

    let path = DerivationPath::from_str("m/44'/0'/0'/0")?;
    let seed = mnemonic.to_seed_normalized(passphrase);
    let root_key = ExtendedPrivKey::new_master(Network::Testnet, &seed)?;
    let extended_key = root_key.into_extended_key()?;
    let xpub = extended_key.into_descriptor_key(None, path).unwrap();

    let (desc, _, _) = bdk::descriptor!(tr(xpub)).unwrap();
    println!("\nBitcoin Configuration");
    println!("  Output Descriptor   : {}", desc.to_string());

    let db = bdk::database::memory::MemoryDatabase::new();
    let wallet = Wallet::new(desc, None, Network::Testnet, db);
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

    Ok(())
}

pub fn print_keys(mnemonic: &Mnemonic, passphrase: &String) -> Result<()> {
    println!("\nMnemonic   : \"{}\" ", &mnemonic.to_string());
    println!("Passphrase : \"{}\" ", &passphrase.to_string());

    print_nostr(&mnemonic, passphrase)?;
    print_bitcoin(&mnemonic, passphrase)?;

    Ok(())
}
