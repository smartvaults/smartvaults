// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use bdk::descriptor::policy::SatisfiableItem;
use coinstr_core::bips::bip39::{self, Mnemonic};
use coinstr_core::bitcoin::Network;
use coinstr_core::miniscript::DescriptorPublicKey;
use coinstr_core::{Policy, PolicyTemplate, SECP256K1};
use keechain_core::types::descriptors::ToDescriptor;
use keechain_core::types::Purpose;
use keechain_core::{Seed, WordCount};

const NETWORK: Network = Network::Testnet;

fn main() {
    let size = 500; // MAX size can be 999

    let mut descriptors: Vec<DescriptorPublicKey> = Vec::new();

    for _ in 0..size {
        let entropy: Vec<u8> = bip39::entropy(WordCount::W24, None);
        let mnemonic = Mnemonic::from_entropy(&entropy).unwrap();
        let seed = Seed::from_mnemonic(mnemonic);
        let desc = seed
            .to_descriptor(Purpose::TR, Some(7291640), false, NETWORK, &SECP256K1)
            .unwrap();
        descriptors.push(desc);
    }

    let template = PolicyTemplate::multisig(size / 2, descriptors);

    let policy = Policy::from_template("", "", template, NETWORK).unwrap();
    println!("Descriptor: {}", policy.descriptor);
    println!(
        "Descriptor size: {} bytes",
        policy.descriptor.to_string().as_bytes().len()
    );
    if let SatisfiableItem::Thresh { items, .. } = policy.satisfiable_item(NETWORK).unwrap() {
        if let SatisfiableItem::Multisig { keys, .. } = &items[1].item {
            println!("Keys in multisig: {}", keys.len());
        }
    }
}
