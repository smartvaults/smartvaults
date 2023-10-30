use std::str::FromStr;

use smartvaults_protocol::nostr::prelude::*;
use smartvaults_protocol::v1::VerifiedKeyAgents;

const BECH32_SECRET_KEY: &str = "";

fn main() -> Result<()> {
    let mut verified_key_agents = VerifiedKeyAgents::empty(Network::Testnet);

    // Add new key agent pubkey
    let public_key = XOnlyPublicKey::from_str(
        "3eea9e831fefdaa8df35187a204d82edb589a36b170955ac5ca6b88340befaa0",
    )?;
    verified_key_agents.add_new_public_key(public_key);

    // Build event
    let secret_key = SecretKey::from_bech32(BECH32_SECRET_KEY)?;
    let keys = Keys::new(secret_key);
    let event = verified_key_agents.to_event(&keys)?;
    println!("{}", event.as_json());

    Ok(())
}
