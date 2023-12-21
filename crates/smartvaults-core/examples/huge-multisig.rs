// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use bdk::descriptor::policy::SatisfiableItem;
use bdk::wallet::AddressIndex;
use bdk::Wallet;
use keechain_core::descriptors::ToDescriptor;
use keechain_core::{Purpose, Seed, WordCount};
use smartvaults_core::bips::bip39::{self, Mnemonic};
use smartvaults_core::bitcoin::Network;
use smartvaults_core::miniscript::DescriptorPublicKey;
use smartvaults_core::{Policy, PolicyTemplate, SECP256K1};

const NETWORK: Network = Network::Testnet;

fn main() {
    let size = 500; // MAX size can be 999

    let mut descriptors: Vec<DescriptorPublicKey> = Vec::new();

    for _ in 0..size {
        let entropy: Vec<u8> = bip39::entropy(WordCount::W24, None);
        let mnemonic = Mnemonic::from_entropy(&entropy).unwrap();
        let seed = Seed::from_mnemonic(mnemonic);
        let desc = seed
            .to_descriptor(Purpose::BIP86, Some(7291640), false, NETWORK, &SECP256K1)
            .unwrap();
        descriptors.push(desc);
    }

    let template = PolicyTemplate::multisig(size / 2, descriptors);

    let policy = Policy::from_template("", "", template, NETWORK).unwrap();
    println!("Descriptor: {}", policy.descriptor());
    println!(
        "Descriptor size: {} bytes",
        policy.as_descriptor().to_string().as_bytes().len()
    );
    if let SatisfiableItem::Thresh { items, .. } = policy.satisfiable_item().unwrap() {
        if let SatisfiableItem::Multisig { keys, .. } = &items[1].item {
            println!("Keys in multisig: {}", keys.len());
        }
    }

    let mut wallet =
        Wallet::new_no_persist(&policy.as_descriptor().to_string(), None, NETWORK).unwrap();
    println!(
        "Receiving address: {}",
        wallet.get_address(AddressIndex::New)
    );
}
