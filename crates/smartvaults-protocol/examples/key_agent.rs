use std::str::FromStr;

use smartvaults_core::bitcoin::Network;
use smartvaults_protocol::nostr::prelude::*;
use smartvaults_protocol::v1::{VerifiedKeyAgentData, VerifiedKeyAgents};

const BECH32_SECRET_KEY: &str = "";

fn main() -> Result<()> {
    let mut verified_key_agents = VerifiedKeyAgents::empty(Network::Testnet);

    // Add new key agent pubkey
    let public_key =
        PublicKey::from_str("3eea9e831fefdaa8df35187a204d82edb589a36b170955ac5ca6b88340befaa0")?;
    let data = VerifiedKeyAgentData {
        approved_at: Some(Timestamp::now()),
    };
    verified_key_agents.add_new_public_key(public_key, data);

    // Build event
    let secret_key = SecretKey::from_bech32(BECH32_SECRET_KEY)?;
    let keys = Keys::new(secret_key);
    let event = verified_key_agents.to_event(&keys)?;
    println!("{}", event.as_json());

    Ok(())
}
