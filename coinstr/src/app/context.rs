// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use coinstr_core::nostr_sdk::EventId;
use coinstr_core::policy::Policy;
use coinstr_core::{Coinstr, CoinstrClient};

use crate::RUNTIME;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Dashboard,
    Policies,
    AddPolicy,
    Policy(EventId, Policy),
    Spend(EventId),
    Proposals,
    Proposal(EventId),
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
}

impl Context {
    pub fn new(stage: Stage, coinstr: Coinstr) -> Self {
        // TODO: let choose the relay and network
        Self {
            stage,
            client: RUNTIME.block_on(async {
                coinstr
                    .client(vec!["wss://relay.rip".to_string()])
                    .await
                    .expect("Impossible to build client")
            }),
            coinstr,
        }
    }

    pub fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
    }
}
