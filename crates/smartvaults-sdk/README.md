# Smart Vaults SDK

## Getting started

```toml
[dependencies]
smartvaults-sdk = { git = "https://github.com/smartvaults/smartvaults", rev = "..." }
tokio = { version = "1", features = ["full"] }
```

```rust,no_run
use std::str::FromStr;

use smartvaults_sdk::core::FeeRate;
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

    // Get first vault
    let vaults = client.vaults().await.unwrap();
    let vault = vaults.first().unwrap();

    // Create a new proposal
    let proposal = client
        .spend(
            &vault.compute_id(),
            Destination::Single(Recipient {
                address: Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78").unwrap().assume_checked(),
                amount: Amount::from_sat(10_934),
            }),
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
    client.approve(&proposal.compute_id(), "password").await.unwrap();
    // other approvals ...

    // Finalize the proposal
    client.finalize(&proposal.compute_id()).await.unwrap();

    // Shutdown the client (for logout)
    client.shutdown().await.unwrap();
}
```

More examples can be found in the [examples/](https://github.com/smartvaults/smartvaults/tree/master/crates/smartvaults-sdk/examples) directory.

Check also the [smartvaults-core](https://github.com/smartvaults/smartvaults/tree/master/crates/smartvaults-core/examples) examples to learn more about templates.
    

## License

This project is distributed under the MIT software license - see the [LICENSE](../../LICENSE) file for details