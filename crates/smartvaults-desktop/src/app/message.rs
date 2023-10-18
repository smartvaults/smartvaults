// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//use super::screen::AddHWSignerMessage;
use super::screen::{
    ActivityMessage, AddAirGapSignerMessage, AddContactMessage, AddNostrConnectSessionMessage,
    AddRelayMessage, AddSignerMessage, AddVaultMessage, AddressesMessage, ChangePasswordMessage,
    CompletedProposalMessage, ConfigMessage, ConnectMessage, ContactsMessage, DashboardMessage,
    EditProfileMessage, HistoryMessage, NewProofMessage, PoliciesMessage, PolicyBuilderMessage,
    PolicyTreeMessage, ProfileMessage, ProposalMessage, ReceiveMessage, RecoveryKeysMessage,
    RelayMessage, RelaysMessage, RestoreVaultMessage, RevokeAllSignersMessage, SelfTransferMessage,
    SettingsMessage, ShareSignerMessage, SignerMessage, SignersMessage, SpendMessage,
    TransactionMessage, VaultMessage,
};
use super::Stage;

#[derive(Debug, Clone)]
pub enum Message {
    View(Stage),
    Dashboard(DashboardMessage),
    Policies(PoliciesMessage),
    AddPolicy(AddVaultMessage),
    PolicyBuilder(PolicyBuilderMessage),
    RestorePolicy(RestoreVaultMessage),
    Policy(VaultMessage),
    PolicyTree(PolicyTreeMessage),
    Spend(SpendMessage),
    Receive(ReceiveMessage),
    SelfTransfer(SelfTransferMessage),
    NewProof(NewProofMessage),
    Activity(ActivityMessage),
    Proposal(ProposalMessage),
    Transaction(TransactionMessage),
    History(HistoryMessage),
    CompletedProposal(CompletedProposalMessage),
    Addresses(AddressesMessage),
    Signers(SignersMessage),
    RevokeAllSigners(RevokeAllSignersMessage),
    Signer(SignerMessage),
    AddSigner(AddSignerMessage),
    //AddHWSigner(AddHWSignerMessage),
    AddAirGapSigner(AddAirGapSignerMessage),
    ShareSigner(ShareSignerMessage),
    Contacts(ContactsMessage),
    AddContact(AddContactMessage),
    Profile(ProfileMessage),
    EditProfile(EditProfileMessage),
    Settings(SettingsMessage),
    Config(ConfigMessage),
    Relays(RelaysMessage),
    Relay(RelayMessage),
    AddRelay(AddRelayMessage),
    ChangePassword(ChangePasswordMessage),
    RecoveryKeys(RecoveryKeysMessage),
    Connect(ConnectMessage),
    AddNostrConnectSession(AddNostrConnectSessionMessage),
    Clipboard(String),
    OpenInBrowser(String),
    ToggleHideBalances,
    Lock,
    Sync,
    Tick,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::App(Box::new(msg))
    }
}
