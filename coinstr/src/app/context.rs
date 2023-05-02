// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bitcoin::Txid;
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::policy::Policy;
use coinstr_core::proposal::Proposal;
use coinstr_core::{Coinstr, CoinstrClient};

use crate::theme::Theme;
use crate::RUNTIME;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Dashboard,
    Policies,
    AddPolicy,
    Policy(EventId),
    Spend(Option<(EventId, Policy)>),
    Receive(Option<(EventId, Policy)>),
    Proposals,
    Proposal(EventId, Proposal),
    Transaction(Txid),
    Transactions(Option<EventId>),
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
    pub theme: Theme,
}

impl Context {
    pub fn new(stage: Stage, coinstr: Coinstr, theme: Theme) -> Self {
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
            client,
            theme,
        }
    }

    pub fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
    }
}
