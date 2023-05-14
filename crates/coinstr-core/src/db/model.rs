// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use nostr_sdk::Timestamp;

use crate::policy::Policy;

#[derive(Debug, Clone)]
pub struct GetPolicyResult {
    pub policy: Policy,
    pub last_sync: Option<Timestamp>,
}
