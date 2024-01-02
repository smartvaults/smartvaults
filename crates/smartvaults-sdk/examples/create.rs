// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::prelude::*;

const NETWORK: Network = Network::Testnet;

#[tokio::main]
async fn main() {
    // Generate and initialize a new client client
    let client = SmartVaults::generate(
        "./your-path",
        "account-name",
        || Ok(String::from("password")),
        || Ok(String::from("confirm-password")),
        WordCount::W24,
        || Ok(None),
        NETWORK,
    )
    .await
    .unwrap();

    // Save default Smart Vaults signer (only the first time)
    let signer_id = client.save_smartvaults_signer().await.unwrap();

    // Get signer by id (or use client.get_signers to get all your signers)
    let signer = client.get_signer_by_id(signer_id).await.unwrap();
    let template = PolicyTemplate::hold(
        signer.descriptor_public_key().unwrap(),
        Locktime::Older(Sequence::from_height(10_000)),
    );

    // Save a new policy from a template
    client
        .save_policy_from_template(
            "My Hold Policy",
            "Policy to keep safe my SATs",
            template,
            vec![client.keys().public_key()],
        )
        .await
        .unwrap();

    // Shutdown the client (for logout)
    client.shutdown().await.unwrap();
}
