use nostr_sdk::Kind;

// Endpoints
pub const DEFAULT_TESTNET_ENDPOINT: &str = "ssl://blockstream.info:993"; // or ssl://electrum.blockstream.info:60002
pub const DEFAULT_BITCOIN_ENDPOINT: &str = "ssl://blockstream.info:700"; // or ssl://electrum.blockstream.info:50002
#[allow(unused)]
pub const DEFAULT_RELAY: &str = "wss://relay.rip";

// Kinds
pub const POLICY_KIND: Kind = Kind::Custom(9289);
pub const SPENDING_PROPOSAL_KIND: Kind = Kind::Custom(9290);
pub const SPENDING_PROPOSAL_APPROVED_KIND: Kind = Kind::Custom(9291);
