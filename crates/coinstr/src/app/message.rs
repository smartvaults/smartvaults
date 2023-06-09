// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use super::screen::{
    AddAirGapSignerMessage, AddHWSignerMessage, AddPolicyMessage, AddSignerMessage,
    CompletedProposalMessage, DashboardMessage, HistoryMessage, NewProofMessage,
    NotificationsMessage, PoliciesMessage, PolicyMessage, ProfileMessage, ProposalMessage,
    ProposalsMessage, ReceiveMessage, RestorePolicyMessage, SettingMessage, SignersMessage,
    SpendMessage, TransactionMessage, TransactionsMessage,
};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Dashboard(DashboardMessage),
    Policies(PoliciesMessage),
    AddPolicy(AddPolicyMessage),
    RestorePolicy(RestorePolicyMessage),
    Policy(PolicyMessage),
    Spend(SpendMessage),
    Receive(ReceiveMessage),
    NewProof(NewProofMessage),
    Proposals(ProposalsMessage),
    Proposal(ProposalMessage),
    Transaction(TransactionMessage),
    Transactions(TransactionsMessage),
    History(HistoryMessage),
    CompletedProposal(CompletedProposalMessage),
    Signers(SignersMessage),
    AddSigner(AddSignerMessage),
    AddHWSigner(AddHWSignerMessage),
    AddAirGapSigner(AddAirGapSignerMessage),
    Notifications(NotificationsMessage),
    Profile(ProfileMessage),
    Setting(SettingMessage),
    Clipboard(String),
    Lock,
    Sync,
    Tick,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::App(Box::new(msg))
    }
}
