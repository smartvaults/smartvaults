// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use smartvaults_sdk::prelude::*;

const NETWORK: Network = Network::Testnet;

#[tokio::main]
async fn main() {
    // Restore and initialize a new Smart Vaults client
    let client = SmartVaults::restore(
        "./your-path",
        "account-name",
        || Ok(String::from("password")),
        || Ok(String::from("confirm-password")),
        || Ok(Mnemonic::from_str("your menmonic").unwrap()),
        || Ok(None),
        NETWORK,
    )
    .await
    .unwrap();

    // Check the others examples to learn more about the client APIs

    // Shutdown the client (for logout)
    client.shutdown().await.unwrap();
}
