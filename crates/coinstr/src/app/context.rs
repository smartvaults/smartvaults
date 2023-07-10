// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;

use coinstr_sdk::core::bitcoin::{Network, Txid};
use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::core::proposal::CompletedProposal;
use coinstr_sdk::core::signer::Signer;
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::{util, Coinstr};

use crate::theme::Theme;
use crate::RUNTIME;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stage {
    Dashboard,
    Policies,
    AddPolicy,
    PolicyBuilder,
    RestorePolicy,
    Policy(EventId),
    Spend(Option<(EventId, Policy)>),
    Receive(Option<(EventId, Policy)>),
    SelfTransfer,
    NewProof(Option<(EventId, Policy)>),
    Proposals,
    Proposal(EventId),
    Transaction(Txid),
    Transactions(Option<EventId>),
    History,
    CompletedProposal(EventId, CompletedProposal, EventId),
    Signers,
    RevokeAllSigners,
    Signer(EventId, Signer),
    AddSigner,
    AddHWSigner,
    AddAirGapSigner,
    ShareSigner(EventId),
    Contacts,
    AddContact,
    Notifications,
    Profile,
    EditProfile,
    Settings,
    Relays,
    NostrConnect,
    AddNostrConnectSession,
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dashboard => write!(f, "Dashboard"),
            Self::Policies => write!(f, "Policies"),
            Self::AddPolicy => write!(f, "Add policy"),
            Self::PolicyBuilder => write!(f, "Builder"),
            Self::RestorePolicy => write!(f, "Restore policy"),
            Self::Policy(id) => write!(f, "Policy #{}", util::cut_event_id(*id)),
            Self::Spend(_) => write!(f, "Spend"),
            Self::Receive(_) => write!(f, "Receive"),
            Self::SelfTransfer => write!(f, "Self transfer"),
            Self::NewProof(_) => write!(f, "New Proof"),
            Self::Proposals => write!(f, "Proposals"),
            Self::Proposal(id) => write!(f, "Proposal #{}", util::cut_event_id(*id)),
            Self::Transaction(txid) => write!(f, "Tx #{}", util::cut_txid(*txid)),
            Self::Transactions(_) => write!(f, "Transactions"),
            Self::History => write!(f, "History"),
            Self::CompletedProposal(..) => write!(f, "Completed proposal"),
            Self::Signers => write!(f, "Signers"),
            Self::RevokeAllSigners => write!(f, "Revoke all"),
            Self::Signer(id, ..) => write!(f, "Signer #{}", util::cut_event_id(*id)),
            Self::AddSigner => write!(f, "Add signer"),
            Self::AddHWSigner => write!(f, "Add HW signer"),
            Self::AddAirGapSigner => write!(f, "Add AirGap signer"),
            Self::ShareSigner(id) => write!(f, "Share signer #{}", util::cut_event_id(*id)),
            Self::Contacts => write!(f, "Contacts"),
            Self::AddContact => write!(f, "Add"),
            Self::Notifications => write!(f, "Notifications"),
            Self::Profile => write!(f, "Profile"),
            Self::EditProfile => write!(f, "Edit profile"),
            Self::Settings => write!(f, "Settings"),
            Self::Relays => write!(f, "Relays"),
            Self::NostrConnect => write!(f, "Connect"),
            Self::AddNostrConnectSession => write!(f, "Add session"),
        }
    }
}

impl Default for Stage {
    fn default() -> Self {
        Self::Dashboard
    }
}

impl Stage {
    pub fn is_breadcrumb_first_level(&self) -> bool {
        matches!(
            self,
            Stage::Dashboard
                | Stage::Policies
                | Stage::Proposals
                | Stage::History
                | Stage::Signers
                | Stage::Contacts
                | Stage::Settings
                | Stage::Notifications
                | Stage::Profile
                | Stage::NostrConnect
        )
    }
}

pub struct Context {
    pub stage: Stage,
    pub client: Coinstr,
    pub theme: Theme,
    pub breadcrumb: Vec<Stage>,
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
        let relays = coinstr.default_relays();
        coinstr.set_electrum_endpoint(endpoint);
        RUNTIME.block_on(async {
            coinstr
                .add_relays_and_connect(relays)
                .await
                .expect("Impossible to build client");
        });

        Self {
            stage: stage.clone(),
            client: coinstr,
            theme,
            breadcrumb: vec![stage],
        }
    }

    pub fn set_stage(&mut self, stage: Stage) {
        if self.breadcrumb.contains(&stage) {
            if let Some(index) = self.breadcrumb.iter().position(|s| s == &stage) {
                self.breadcrumb = self.breadcrumb.clone().into_iter().take(index).collect();
            }
        }
        self.breadcrumb.push(stage.clone());
        self.stage = stage;
    }

    pub fn reset_breadcrumb(&mut self) {
        self.breadcrumb.clear();
    }
}
