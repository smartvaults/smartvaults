// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use smartvaults_sdk::core::{Amount, FeeRate};
use smartvaults_sdk::prelude::*;

const NETWORK: Network = Network::Testnet;

#[tokio::main]
async fn main() {
    // Open a keychain and init the client (check the other examples the learn how to create or restore a client)
    let client = SmartVaults::open("./your-path", "account-name", "password", NETWORK)
        .await
        .unwrap();

    // Edit relays
    client.add_relay("wss://you.relay.com", None).await.unwrap();

    // Edit configs
    let config = client.config();
    config
        .set_electrum_endpoint(Some("tcp://127.0.0.1:50001"))
        .await
        .unwrap();
    config
        .set_block_explorer(Some(Url::parse("http://myblockexplorer.local").unwrap()))
        .await;
    config.save().await.unwrap();

    // Get policies
    let policies = client.get_policies().await.unwrap();
    for policy in policies.iter() {
        println!("{policy:?}");
    }

    // Create a new proposal
    let proposal = client
        .spend(
            policies.first().unwrap().policy_id,
            Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78").unwrap(),
            Amount::Custom(10_934), // Or, `Amount::Max` to send all
            "Back to the faucet",
            FeeRate::Priority(Priority::Medium), // Or, FeeRate::Rate(1.0) to specify the sat/vByte
            None,                                // Specify the UTXOs to use (optional)
            None, // Specify the policy path to use (needed only if exists a timelock in the policy descriptor)
            false, // Allow usage of UTXOs frozen by others proposals
        )
        .await
        .unwrap();
    println!("New proposal: {proposal:#?}");

    // Get proposals
    let proposals = client.get_proposals().await.unwrap();
    for proposal in proposals.into_iter() {
        println!("{proposal:?}");
    }

    // Approve a proposal
    client
        .approve("password", proposal.proposal_id)
        .await
        .unwrap();
    // other approvals ...

    // Finalize the proposal
    client.finalize(proposal.proposal_id).await.unwrap();

    // Shutdown the client (for logout)
    client.shutdown().await.unwrap();
}
