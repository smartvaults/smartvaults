use std::collections::HashMap;

use coinstr_core::bdk::database::MemoryDatabase;
use coinstr_core::bdk::wallet::AddressIndex;
use coinstr_core::bdk::Wallet;
use coinstr_core::bitcoin::util::bip32::ExtendedPubKey;
use coinstr_core::bitcoin::Network;
use coinstr_core::nostr_sdk::prelude::{ToBech32, XOnlyPublicKey};
use coinstr_core::nostr_sdk::{Metadata, SECP256K1};
use coinstr_core::types::Purpose;
use coinstr_core::util::bip::bip32::Bip32RootKey;
use coinstr_core::{Keychain, Result};
use prettytable::{row, Table};

pub fn print_secrets(keychain: Keychain, network: Network) -> Result<()> {
    let mnemonic = keychain.seed.mnemonic();
    let passphrase = keychain.seed.passphrase();

    println!();

    println!("Mnemonic: {}", mnemonic);
    if let Some(passphrase) = passphrase {
        println!("Passphrase: {}", passphrase);
    }

    let keys = keychain.nostr_keys()?;

    println!("\nNostr");
    println!(" Bech32 Keys");
    println!("  Public   : {} ", keys.public_key().to_bech32()?);
    println!("  Private  : {} ", keys.secret_key()?.to_bech32()?);
    println!(" Hex Keys");
    println!("  Public   : {} ", keys.public_key());
    println!("  Private  : {} ", keys.secret_key()?.display_secret());

    let root_key = keychain.seed.to_bip32_root_key(network)?;
    let descriptors = keychain.descriptors(network, None)?;
    let external = descriptors.get_by_purpose(Purpose::TR, false).unwrap();
    let internal = descriptors.get_by_purpose(Purpose::TR, true).unwrap();
    let wallet = Wallet::new(
        external.clone(),
        Some(internal.clone()),
        network,
        MemoryDatabase::new(),
    )
    .unwrap();

    println!("\nBitcoin");
    println!("  Root Private Key: {root_key}");
    println!(
        "  Extended Pub Key: {}",
        ExtendedPubKey::from_priv(SECP256K1, &root_key)
    );
    println!("  Output Descriptor: {external}");
    println!("  Change Descriptor: {internal}");
    println!(
        "  Ext Address 1: {}",
        wallet.get_address(AddressIndex::New).unwrap()
    );
    println!(
        "  Ext Address 2: {}",
        wallet.get_address(AddressIndex::New).unwrap()
    );
    println!(
        "  Change Address: {}",
        wallet.get_internal_address(AddressIndex::New).unwrap()
    );

    Ok(())
}

pub fn print_contacts(contacts: HashMap<XOnlyPublicKey, Metadata>) {
    let mut table = Table::new();

    table.set_titles(row![
        "#",
        "Public key",
        "Username",
        "Display name",
        "NIP-05",
    ]);

    for (index, (public_key, metadata)) in contacts.into_iter().enumerate() {
        table.add_row(row![
            index + 1,
            public_key,
            metadata.name.unwrap_or_default(),
            metadata.display_name.unwrap_or_default(),
            metadata.nip05.unwrap_or_default()
        ]);
    }

    table.printstd();
}
