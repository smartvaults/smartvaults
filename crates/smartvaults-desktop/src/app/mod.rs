// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::{clipboard, Command, Element, Subscription};
use smartvaults_sdk::core::bitcoin::Network;
use smartvaults_sdk::{Message as SdkMessage, SmartVaults};

mod component;
mod context;
mod message;
pub mod screen;
mod sync;

pub use self::context::{Context, Stage};
pub use self::message::Message;
use self::screen::*;
use self::sync::SmartVaultsSync;
use crate::theme::Theme;

pub trait State {
    fn title(&self) -> String;

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message>;

    fn view(&self, ctx: &Context) -> Element<Message>;

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn load(&mut self, _ctx: &Context) -> Command<Message> {
        Command::none()
    }
}

pub fn new_state(ctx: &Context) -> Box<dyn State> {
    match &ctx.stage {
        Stage::Dashboard => DashboardState::new().into(),
        Stage::Vaults => PoliciesState::new().into(),
        Stage::AddVault => AddVaultState::new().into(),
        Stage::VaultBuilder => PolicyBuilderState::new().into(),
        Stage::RestoreVault => RestoreVaultState::new().into(),
        Stage::Vault(policy_id) => VaultState::new(*policy_id).into(),
        Stage::PolicyTree(policy_id) => PolicyTreeState::new(*policy_id).into(),
        Stage::Spend(policy) => SpendState::new(policy.clone()).into(),
        Stage::Receive(policy) => ReceiveState::new(policy.clone()).into(),
        Stage::NewProof(policy) => NewProofState::new(policy.clone()).into(),
        Stage::Activity => ActivityState::new().into(),
        Stage::Proposal(proposal_id) => ProposalState::new(*proposal_id).into(),
        Stage::Transaction { vault_id, txid } => TransactionState::new(*vault_id, *txid).into(),
        Stage::Addresses(policy) => AddressesState::new(policy.clone()).into(),
        Stage::Signers => SignersState::new().into(),
        Stage::RevokeAllSigners => RevokeAllSignersState::new().into(),
        Stage::Signer(signer_id, signer) => SignerState::new(*signer_id, signer.clone()).into(),
        Stage::AddSigner => AddSignerState::new().into(),
        // Stage::AddHWSigner => AddHWSignerState::new().into(),
        Stage::AddAirGapSigner => AddAirGapSignerState::new().into(),
        Stage::AddColdcardSigner => AddColdcardSignerState::new().into(),
        Stage::ShareSigner(signer_id) => ShareSignerState::new(*signer_id).into(),
        Stage::EditSignerOffering(signer) => EditSignerOfferingState::new(signer.clone()).into(),
        Stage::KeyAgents => KeyAgentsState::new().into(),
        Stage::Contacts => ContactsState::new().into(),
        Stage::AddContact => AddContactState::new().into(),
        Stage::Profile => ProfileState::new().into(),
        Stage::EditProfile => EditProfileState::new().into(),
        Stage::Settings => SettingsState::new().into(),
        Stage::Relays => RelaysState::new().into(),
        Stage::Relay(url) => RelayState::new(url.clone()).into(),
        Stage::Config => ConfigState::new().into(),
        Stage::AddRelay => AddRelayState::new().into(),
        Stage::ChangePassword => ChangePasswordState::new().into(),
        Stage::RecoveryKeys => RecoveryKeysState::new().into(),
        Stage::WipeKeys => WipeKeysState::new().into(),
        Stage::NostrConnect => ConnectState::new().into(),
        Stage::AddNostrConnectSession => AddNostrConnectSessionState::new().into(),
    }
}

pub struct App {
    state: Box<dyn State>,
    pub(crate) ctx: Context,
}

impl App {
    pub fn new(client: SmartVaults) -> Self {
        let stage = Stage::default();
        let ctx = Context::new(stage, client);
        Self {
            state: new_state(&ctx),
            ctx,
        }
    }

    pub fn title(&self) -> String {
        match self.ctx.client.name() {
            Some(name) => format!("{} [{name}]", self.state.title()),
            None => self.state.title(),
        }
    }

    pub fn theme(&self) -> Theme {
        match self.ctx.client.network() {
            Network::Bitcoin => Theme::Mainnet,
            Network::Testnet => Theme::Testnet,
            Network::Signet => Theme::Signet,
            _ => Theme::Regtest,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let sync = SmartVaultsSync::subscription(self.ctx.client.clone()).map(Message::Sync);
        Subscription::batch(vec![sync, self.state.subscription()])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::View(stage) => {
                if stage.is_breadcrumb_first_level() {
                    self.ctx.reset_breadcrumb();
                }
                self.ctx.set_stage(stage);
                self.state = new_state(&self.ctx);
                self.state.load(&self.ctx)
            }
            Message::Tick => self.state.update(&mut self.ctx, message),
            Message::Sync(msg) => match msg {
                SdkMessage::MempoolFeesUpdated(fees) => {
                    self.ctx.current_fees = fees;
                    Command::none()
                }
                _ => self.state.load(&self.ctx),
            },
            Message::Clipboard(data) => clipboard::write(data),
            Message::OpenInBrowser(url) => {
                if let Err(e) = webbrowser::open(&url) {
                    tracing::error!("Impossible to open link on browser: {e}");
                }
                Command::none()
            }
            Message::ChangeMode(mode) => {
                self.ctx.set_mode(mode);
                self.state.load(&self.ctx)
            }
            Message::ToggleHideBalances => {
                self.ctx.toggle_hide_balances();
                Command::none()
            }
            _ => self.state.update(&mut self.ctx, message),
        }
    }

    pub fn view(&self) -> Element<Message> {
        self.state.view(&self.ctx)
    }
}
