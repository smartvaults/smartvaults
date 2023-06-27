// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use super::screen::{
    AddAirGapSignerMessage, AddContactMessage, AddHWSignerMessage, AddPolicyMessage,
    AddSignerMessage, CompletedProposalMessage, ContactsMessage, DashboardMessage,
    EditProfileMessage, HistoryMessage, NewProofMessage, NotificationsMessage, PoliciesMessage,
    PolicyBuilderMessage, PolicyMessage, ProfileMessage, ProposalMessage, ProposalsMessage,
    ReceiveMessage, RestorePolicyMessage, SelfTransferMessage, SettingMessage, ShareSignerMessage,
    SignerMessage, SignersMessage, SpendMessage, TransactionMessage, TransactionsMessage,
};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Dashboard(DashboardMessage),
    Policies(PoliciesMessage),
    AddPolicy(AddPolicyMessage),
    PolicyBuilder(PolicyBuilderMessage),
    RestorePolicy(RestorePolicyMessage),
    Policy(PolicyMessage),
    Spend(SpendMessage),
    Receive(ReceiveMessage),
    SelfTransfer(SelfTransferMessage),
    NewProof(NewProofMessage),
    Proposals(ProposalsMessage),
    Proposal(ProposalMessage),
    Transaction(TransactionMessage),
    Transactions(TransactionsMessage),
    History(HistoryMessage),
    CompletedProposal(CompletedProposalMessage),
    Signers(SignersMessage),
    Signer(SignerMessage),
    AddSigner(AddSignerMessage),
    AddHWSigner(AddHWSignerMessage),
    AddAirGapSigner(AddAirGapSignerMessage),
    ShareSigner(ShareSignerMessage),
    Contacts(ContactsMessage),
    AddContact(AddContactMessage),
    Notifications(NotificationsMessage),
    Profile(ProfileMessage),
    EditProfile(EditProfileMessage),
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
