// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_sdk::prelude::*;

const NETWORK: Network = Network::Testnet;

#[tokio::main]
async fn main() {
    // Restore and initialize a new coinstr client
    let coinstr = Coinstr::restore(
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
    coinstr.shutdown().await.unwrap();
}
