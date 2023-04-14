// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use coinstr_core::nostr_sdk::EventId;
use coinstr_core::policy::Policy;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::{Coinstr, CoinstrClient};

use super::cache::Cache;
use crate::{APP_PATH, RUNTIME};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Dashboard,
    Policies,
    AddPolicy,
    Policy(EventId, Policy),
    Spend(EventId),
    Proposals,
    Proposal(EventId, SpendingProposal),
    Setting,
}

impl Default for Stage {
    fn default() -> Self {
        Self::Dashboard
    }
}

pub struct Context {
    pub stage: Stage,
    pub coinstr: Coinstr,
    pub client: CoinstrClient,
    pub cache: Cache,
}

impl Context {
    pub fn new(stage: Stage, coinstr: Coinstr) -> Self {
        // TODO: let choose the relay and network
        let client = RUNTIME.block_on(async {
            coinstr
                .client(vec!["wss://relay.rip".to_string()])
                .await
                .expect("Impossible to build client")
        });

        Self {
            stage,
            coinstr,
            cache: Cache::new(
                APP_PATH
                    .join("cache")
                    .join(client.inner().keys().public_key().to_string()),
            ),
            client,
        }
    }

    pub fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
    }
}
