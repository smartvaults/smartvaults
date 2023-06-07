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
    AddPolicyState, CompletedProposalState, DashboardState, HistoryState, NewProofState,
    NotificationsState, PoliciesState, PolicyState, ProposalState, ProposalsState, ReceiveState,
    RestorePolicyState, SettingState, SpendState, TransactionState, TransactionsState,
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
        Stage::Notifications => NotificationsState::new().into(),
        Stage::Setting => SettingState::new().into(),
    }
}

pub struct App {
    state: Box<dyn State>,
    pub(crate) ctx: Context,
}

impl App {
    pub fn new(coinstr: Coinstr, theme: Theme) -> (Self, Command<Message>) {
        let stage = Stage::default();
        let ctx = Context::new(stage.clone(), coinstr, theme);
        let app = Self {
            state: new_state(&ctx),
            ctx,
        };
        (
            app,
            Command::perform(async {}, move |_| Message::View(stage)),
        )
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
            Message::Sync => self.state.load(&self.ctx),
            Message::Clipboard(data) => clipboard::write(data),
            _ => self.state.update(&mut self.ctx, message),
        }
    }

    pub fn view(&self) -> Element<Message> {
        self.state.view(&self.ctx)
    }
}
