// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use super::screen::{
    AddPolicyMessage, DashboardMessage, PoliciesMessage, PolicyMessage, ProposalMessage,
    ProposalsMessage, SpendMessage,
};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Dashboard(DashboardMessage),
    Policies(PoliciesMessage),
    AddPolicy(AddPolicyMessage),
    Policy(PolicyMessage),
    Spend(SpendMessage),
    Proposals(ProposalsMessage),
    Proposal(ProposalMessage),
    Lock,
    Sync,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::App(Box::new(msg))
    }
}
