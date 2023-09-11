// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::time::Duration;

// Default relays
pub const MAINNET_RELAYS: [&str; 1] = ["wss://prod.relay.report"];
pub const TESTNET_RELAYS: [&str; 1] = ["wss://test.relay.report"];

// Sync intervals
pub const BLOCK_HEIGHT_SYNC_INTERVAL: Duration = Duration::from_secs(60);
pub const WALLET_SYNC_INTERVAL: Duration = Duration::from_secs(60);
pub const METADATA_SYNC_INTERVAL: Duration = Duration::from_secs(3600);

// Timeout
pub(crate) const SEND_TIMEOUT: Duration = Duration::from_secs(20);
