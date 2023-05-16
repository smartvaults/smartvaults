// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use bdk::Balance;
use nostr_sdk::Timestamp;

use crate::policy::Policy;

#[derive(Debug, Clone)]
pub struct GetPolicyResult {
    pub policy: Policy,
    pub last_sync: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct GetDetailedPolicyResult {
    pub policy: Policy,
    pub balance: Option<Balance>,
    pub last_sync: Option<Timestamp>,
}
