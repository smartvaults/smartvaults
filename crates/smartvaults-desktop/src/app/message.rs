// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::Message as SdkMessage;

use super::context::Mode;
// use super::screen::AddHWSignerMessage;
use super::screen::{
    ActivityMessage, AddAirGapSignerMessage, AddColdcardSignerMessage, AddContactMessage,
    AddNostrConnectSessionMessage, AddRelayMessage, AddSignerMessage, AddVaultMessage,
    AddressesMessage, ChangePasswordMessage, CompletedProposalMessage, ConfigMessage,
    ConnectMessage, ContactsMessage, DashboardMessage, EditProfileMessage,
    EditSignerOfferingMessage, HistoryMessage, KeyAgentsMessage, NewProofMessage, PoliciesMessage,
    PolicyBuilderMessage, PolicyTreeMessage, ProfileMessage, ProposalMessage, ReceiveMessage,
    RecoveryKeysMessage, RelayMessage, RelaysMessage, RestoreVaultMessage, RevokeAllSignersMessage,
    SelfTransferMessage, SettingsMessage, ShareSignerMessage, SignerMessage, SignersMessage,
    SpendMessage, TransactionMessage, VaultMessage, WipeKeysMessage,
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
    // AddHWSigner(AddHWSignerMessage),
    AddAirGapSigner(AddAirGapSignerMessage),
    AddColdcardSigner(AddColdcardSignerMessage),
    ShareSigner(ShareSignerMessage),
    EditSignerOffering(EditSignerOfferingMessage),
    KeyAgents(KeyAgentsMessage),
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
    WipeKeys(WipeKeysMessage),
    Connect(ConnectMessage),
    AddNostrConnectSession(AddNostrConnectSessionMessage),
    Clipboard(String),
    OpenInBrowser(String),
    ChangeMode(Mode),
    ToggleHideBalances,
    Lock,
    Sync(SdkMessage),
    Tick,
}

impl From<Message> for crate::Message {
    fn from(msg: Message) -> Self {
        Self::App(Box::new(msg))
    }
}
