// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::time::Duration;

// Default relays
pub const MAINNET_RELAYS: [&str; 2] = [
    "wss://relay-prod1.hashed.systems",
    "wss://relay-prod2.hashed.systems",
];
pub const TESTNET_RELAYS: [&str; 2] = [
    "wss://relay-test1.hashed.systems",
    "wss://relay-test2.hashed.systems",
];

// Sync intervals
pub const BLOCK_HEIGHT_SYNC_INTERVAL: Duration = Duration::from_secs(60);
pub const MEMPOOL_TX_FEES_SYNC_INTERVAL: Duration = Duration::from_secs(60);
pub const WALLET_SYNC_INTERVAL: Duration = Duration::from_secs(60);
pub const METADATA_SYNC_INTERVAL: Duration = Duration::from_secs(3600);

// Timeout
pub(crate) const SEND_TIMEOUT: Duration = Duration::from_secs(20);

pub(crate) const DEFAULT_SUBSCRIPTION_ID: &str = "smartvaults";
pub(crate) const NOSTR_CONNECT_SUBSCRIPTION_ID: &str = "ncs";
