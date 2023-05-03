// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use super::screen::{
    AddPolicyMessage, DashboardMessage, HistoryMessage, NewProofMessage, PoliciesMessage,
    PolicyMessage, ProposalMessage, ProposalsMessage, ReceiveMessage, SettingMessage, SpendMessage,
    TransactionMessage, TransactionsMessage,
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
    Receive(ReceiveMessage),
    NewProof(NewProofMessage),
    Proposals(ProposalsMessage),
    Proposal(ProposalMessage),
    Transaction(TransactionMessage),
    Transactions(TransactionsMessage),
    History(HistoryMessage),
    Setting(SettingMessage),
    Clipboard(String),
    Lock,
    Sync,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::App(Box::new(msg))
    }
}
