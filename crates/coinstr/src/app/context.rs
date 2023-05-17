// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bitcoin::{Network, Txid};
use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::core::proposal::{CompletedProposal, Proposal};
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::Coinstr;

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
    NewProof(Option<(EventId, Policy)>),
    Proposals,
    Proposal(EventId, Proposal, EventId),
    Transaction(Txid),
    Transactions(Option<EventId>),
    History,
    CompletedProposal(EventId, CompletedProposal, EventId),
    Setting,
}

impl Default for Stage {
    fn default() -> Self {
        Self::Dashboard
    }
}

pub struct Context {
    pub stage: Stage,
    pub client: Coinstr,
    pub theme: Theme,
}

impl Context {
    pub fn new(stage: Stage, coinstr: Coinstr, theme: Theme) -> Self {
        // TODO: let choose the relay, network and electrum endpoint
        let endpoint: &str = match coinstr.network() {
            Network::Bitcoin => "ssl://blockstream.info:700",
            Network::Testnet => "ssl://blockstream.info:993",
            Network::Signet => "tcp://signet-electrumx.wakiyamap.dev:50001",
            Network::Regtest => "tcp://localhost:60401",
        };
        coinstr.set_electrum_endpoint(endpoint);
        RUNTIME.block_on(async {
            coinstr
                .add_relays_and_connect(vec!["wss://relay.rip".to_string()])
                .await
                .expect("Impossible to build client");
        });

        Self {
            stage,
            client: coinstr,
            theme,
        }
    }

    pub fn set_stage(&mut self, stage: Stage) {
        self.stage = stage;
    }
}
