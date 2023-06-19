// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::Coinstr;
use iced::{clipboard, Command, Element, Subscription};

mod component;
mod context;
mod message;
pub mod screen;
mod sync;

use crate::constants::APP_NAME;
use crate::theme::Theme;

pub use self::context::{Context, Stage};
pub use self::message::Message;
use self::screen::{
    AddAirGapSignerState, AddHWSignerState, AddPolicyState, AddSignerState, CompletedProposalState,
    ContactsState, DashboardState, HistoryState, NewProofState, NotificationsState, PoliciesState,
    PolicyState, ProfileState, ProposalState, ProposalsState, ReceiveState, RestorePolicyState,
    SelfTransferState, SettingState, SignerState, SignersState, SpendState, TransactionState,
    TransactionsState,
};
use self::sync::CoinstrSync;

pub trait State {
    fn title(&self) -> String {
        APP_NAME.to_string()
    }

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
        Stage::Policies => PoliciesState::new().into(),
        Stage::AddPolicy => AddPolicyState::new().into(),
        Stage::RestorePolicy => RestorePolicyState::new().into(),
        Stage::Policy(policy_id) => PolicyState::new(*policy_id).into(),
        Stage::Spend(policy) => SpendState::new(policy.clone()).into(),
        Stage::Receive(policy) => ReceiveState::new(policy.clone()).into(),
        Stage::SelfTransfer => SelfTransferState::new().into(),
        Stage::NewProof(policy) => NewProofState::new(policy.clone()).into(),
        Stage::Proposals => ProposalsState::new().into(),
        Stage::Proposal(proposal_id) => ProposalState::new(*proposal_id).into(),
        Stage::Transaction(txid) => TransactionState::new(*txid).into(),
        Stage::Transactions(policy_id) => TransactionsState::new(*policy_id).into(),
        Stage::History => HistoryState::new().into(),
        Stage::CompletedProposal(completed_proposal_id, completed_proposal, policy_id) => {
            CompletedProposalState::new(
                *completed_proposal_id,
                completed_proposal.clone(),
                *policy_id,
            )
            .into()
        }
        Stage::Signers => SignersState::new().into(),
        Stage::Signer(signer_id, signer) => SignerState::new(*signer_id, signer.clone()).into(),
        Stage::AddSigner => AddSignerState::new().into(),
        Stage::AddHWSigner => AddHWSignerState::new().into(),
        Stage::AddAirGapSigner => AddAirGapSignerState::new().into(),
        Stage::Contacts => ContactsState::new().into(),
        Stage::Notifications => NotificationsState::new().into(),
        Stage::Profile => ProfileState::new().into(),
        Stage::Setting => SettingState::new().into(),
    }
}

pub struct App {
    state: Box<dyn State>,
    pub(crate) ctx: Context,
}

impl App {
    pub fn new(coinstr: Coinstr, theme: Theme) -> Self {
        let stage = Stage::default();
        let ctx = Context::new(stage, coinstr, theme);
        let app = Self {
            state: new_state(&ctx),
            ctx,
        };
        app
    }

    pub fn title(&self) -> String {
        self.state.title()
    }

    pub fn theme(&self) -> Theme {
        self.ctx.theme
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let sync = CoinstrSync::subscription(self.ctx.client.clone()).map(|_| Message::Sync);
        Subscription::batch(vec![sync, self.state.subscription()])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::View(stage) => {
                self.ctx.set_stage(stage);
                self.state = new_state(&self.ctx);
                self.state.load(&self.ctx)
            }
            Message::Tick => self.state.update(&mut self.ctx, message),
            Message::Sync => self.state.load(&self.ctx),
            Message::Clipboard(data) => clipboard::write(data),
            _ => self.state.update(&mut self.ctx, message),
        }
    }

    pub fn view(&self) -> Element<Message> {
        self.state.view(&self.ctx)
    }
}
