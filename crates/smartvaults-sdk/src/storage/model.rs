// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::Timestamp;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::Policy;

#[derive(Debug, Clone)]
pub(crate) struct InternalPolicy {
    pub policy: Policy,
    pub public_keys: Vec<XOnlyPublicKey>,
    pub last_sync: Option<Timestamp>,
}
