# Coinstr SDK

## Getting started

```toml
[dependencies]
coinstr-sdk = { git = "https://github.com/smartvaults/coinstr", rev = "..." }
tokio = { version = "1", features = ["full"] }
```

```rust,no_run
use std::str::FromStr;

use coinstr_sdk::core::{Amount, FeeRate};
use coinstr_sdk::prelude::*;

const NETWORK: Network = Network::Testnet;

#[tokio::main]
async fn main() {
    // Open a keychain and init the client (check the other examples the learn how to create or restore a client)
    let coinstr = Coinstr::open("./your-path", "account-name", || Ok(String::from("password")), NETWORK)
        .await
        .unwrap();

    // Edit relays
    coinstr
        .add_relay("wss://you.relay.com", None)
        .await
        .unwrap();

    // Edit configs
    let config = coinstr.config();
    config
        .set_electrum_endpoint(Some("tcp://127.0.0.1:50001"))
        .await;
    config
        .set_block_explorer(Some(Url::parse("http://myblockexplorer.local").unwrap()))
        .await;
    config.save().await.unwrap();

    // Create a new proposal
    let policies = coinstr.get_policies().await.unwrap();
    let proposal = coinstr
        .spend(
            policies.first().unwrap().policy_id,
            Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78").unwrap(),
            Amount::Custom(10_934), // Or, `Amount::Max` to send all
            "Back to the faucet",
            FeeRate::Priority(Priority::Medium), // Or, FeeRate::Rate(1.0) to specify the sat/vByte
            None, // Specify the UTXOs to use (optional)
            None, // Specify the policy path to use (needed only if exists a timelock in the policy descriptor)
            false, // Allow usage of UTXOs frozen by others proposals
        )
        .await
        .unwrap();
    println!("New proposal: {proposal:#?}");

    // Approve a proposal
    coinstr.approve(proposal.proposal_id).await.unwrap();
    // other approvals ...

    // Finalize the proposal
    coinstr.finalize(proposal.proposal_id).await.unwrap();

    // Shutdown the client (for logout)
    coinstr.shutdown().await.unwrap();
}
```

More examples can be found in the [examples/](https://github.com/smartvaults/coinstr/tree/master/crates/coinstr-sdk/examples) directory.

Check also the [coinstr-core](https://github.com/smartvaults/coinstr/tree/master/crates/coinstr-core/examples) examples to learn more about templates.
    

## License

This project is distributed under the MIT software license - see the [LICENSE](../../LICENSE) file for details