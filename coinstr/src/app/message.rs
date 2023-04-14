// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use super::screen::{
    AddPolicyMessage, PoliciesMessage, PolicyMessage, ProposalsMessage, SpendMessage,
};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Policies(PoliciesMessage),
    AddPolicy(AddPolicyMessage),
    Policy(PolicyMessage),
    Spend(SpendMessage),
    Proposals(ProposalsMessage),
    Lock,
    Sync,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::App(Box::new(msg))
    }
}
