// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

#[cfg(feature = "hwi")]
use super::screen::AddHWSignerMessage;
use super::screen::{
    AddAirGapSignerMessage, AddContactMessage, AddNostrConnectSessionMessage, AddPolicyMessage,
    AddSignerMessage, CompletedProposalMessage, ConnectMessage, ContactsMessage, DashboardMessage,
    EditProfileMessage, HistoryMessage, NewProofMessage, NotificationsMessage, PoliciesMessage,
    PolicyBuilderMessage, PolicyMessage, ProfileMessage, ProposalMessage, ProposalsMessage,
    ReceiveMessage, RelaysMessage, RestorePolicyMessage, RevokeAllSignersMessage,
    SelfTransferMessage, SettingsMessage, ShareSignerMessage, SignerMessage, SignersMessage,
    SpendMessage, TransactionMessage, TransactionsMessage,
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
    RevokeAllSigners(RevokeAllSignersMessage),
    Signer(SignerMessage),
    AddSigner(AddSignerMessage),
    #[cfg(feature = "hwi")]
    AddHWSigner(AddHWSignerMessage),
    AddAirGapSigner(AddAirGapSignerMessage),
    ShareSigner(ShareSignerMessage),
    Contacts(ContactsMessage),
    AddContact(AddContactMessage),
    Notifications(NotificationsMessage),
    Profile(ProfileMessage),
    EditProfile(EditProfileMessage),
    Settings(SettingsMessage),
    Relays(RelaysMessage),
    Connect(ConnectMessage),
    AddNostrConnectSession(AddNostrConnectSessionMessage),
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
