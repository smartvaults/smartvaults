// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::prelude::*;

const NETWORK: Network = Network::Testnet;

#[tokio::main]
async fn main() {
    // Generate and initialize a new coinstr client
    let coinstr = Coinstr::generate(
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

    // Save default coinstr signer (only the first time)
    let signer_id = coinstr.save_coinstr_signer().await.unwrap();

    // Get signer by id (or use coinstr.get_signers to get all your signers)
    let signer = coinstr.get_signer_by_id(signer_id).await.unwrap();
    let template = PolicyTemplate::hold(
        signer.descriptor_public_key().unwrap(),
        Sequence::from_height(10_000),
    );

    // Save a new policy from a template
    coinstr
        .save_policy_from_template(
            "My Hold Policy",
            "Policy to keep safe my SATs",
            template,
            vec![coinstr.keys().public_key()],
        )
        .await
        .unwrap();

    // Shutdown the client (for logout)
    coinstr.shutdown().await.unwrap();
}
