// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use smartvaults_protocol::core::bitcoin::Network;
use smartvaults_protocol::core::miniscript::DescriptorPublicKey;
use smartvaults_protocol::core::PolicyTemplate;
use smartvaults_protocol::nostr::{Keys, SecretKey};
use smartvaults_protocol::v2::{ProtocolEncryption, Vault};

const SECRET_KEY: &str = "6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e";

fn main() {
    // Descriptors
    let desc1 = DescriptorPublicKey::from_str("[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*").unwrap();
    let desc2 = DescriptorPublicKey::from_str("[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*").unwrap();

    // Multisig 2 of 2
    let template = PolicyTemplate::multisig(2, vec![desc1, desc2]);

    // Compose vault
    let network = Network::Testnet;
    let shared_key = Keys::generate();
    let vault = Vault::from_template(
        template.clone(),
        network,
        shared_key.secret_key().unwrap().clone(),
    )
    .unwrap();

    // Encryption keys
    let secret_key = SecretKey::from_str(SECRET_KEY).unwrap();
    let keys = Keys::new(secret_key);

    println!("Descriptor: {}", vault.descriptor());
    println!(
        "Encrypted vault: {}",
        vault.encrypt_with_keys(&keys).unwrap()
    );
}
