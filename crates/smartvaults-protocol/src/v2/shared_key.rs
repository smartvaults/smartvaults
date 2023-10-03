// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr::event::builder::Error;
use nostr::{Event, EventBuilder, EventId, Keys, Tag, SecretKey, PublicKey};
use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::Network;

use super::constants::SHARED_KEY_KIND_V2;
use super::crypto::{self, Version as CryptoVersion};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SharedKey {
    V1 {
        /// Secret key
        shared_key: SecretKey,
        /// Event ID
        policy_id: EventId,
        /// Network magic
        network: [u8; 4],
    },
}

pub fn build_event(
    keys: &Keys,
    receiver: &PublicKey,
    shared_key: &Keys,
    policy_id: EventId,
    network: Network,
) -> Result<Event, Error> {
    // Compose Shared Key
    let shared_key = SharedKey::V1 {
        shared_key: shared_key.secret_key()?.clone(),
        policy_id,
        network: network.magic().to_bytes(),
    };

    // Encrypt Shared Key
    let encrypted_shared_key = crypto::encrypt(
        keys.secret_key()?,
        receiver,
        serde_json::to_vec(&shared_key)?, // TODO: avoid to use JSON?
        CryptoVersion::XChaCha20Poly1305,
    )?;

    // Compose and build event
    Ok(EventBuilder::new(
        SHARED_KEY_KIND_V2,
        encrypted_shared_key,
        // Include only the public key able to decrypt the event to avoid leak of other data
        [Tag::public_key(*receiver)],
    )
    .to_event(keys)?)
}
