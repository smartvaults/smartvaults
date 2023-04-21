// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::Coinstr;
use iced::{clipboard, Command, Element, Subscription};

mod cache;
mod component;
mod context;
mod message;
pub mod screen;
mod sync;

use crate::constants::APP_NAME;

pub use self::context::{Context, Stage};
pub use self::message::Message;
use self::screen::{
    AddPolicyState, DashboardState, PoliciesState, PolicyState, ProposalState, ProposalsState,
    ReceiveState, SettingState, SpendState, TransactionState, TransactionsState,
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
        Stage::Policy(policy_id) => PolicyState::new(*policy_id).into(),
        Stage::Spend(policy) => SpendState::new(policy.clone()).into(),
        Stage::Receive(policy) => ReceiveState::new(policy.clone()).into(),
        Stage::Proposals => ProposalsState::new().into(),
        Stage::Proposal(proposal_id, proposal) => {
            ProposalState::new(*proposal_id, proposal.clone()).into()
        }
        Stage::Transaction(txid) => TransactionState::new(*txid).into(),
        Stage::Transactions(policy_id) => TransactionsState::new(*policy_id).into(),
        Stage::Setting => SettingState::new().into(),
    }
}

pub struct App {
    state: Box<dyn State>,
    pub(crate) context: Context,
}

impl App {
    pub fn new(coinstr: Coinstr) -> (Self, Command<Message>) {
        let stage = Stage::default();
        let context = Context::new(stage.clone(), coinstr);
        let app = Self {
            state: new_state(&context),
            context,
        };
        (
            app,
            Command::perform(async {}, move |_| Message::View(stage)),
        )
    }

    pub fn title(&self) -> String {
        self.state.title()
    }

    pub fn subscription(&self) -> Subscription<Message> {
        let sync =
            CoinstrSync::subscription(self.context.client.clone(), self.context.cache.clone())
                .map(|_| Message::Sync);
        Subscription::batch(vec![sync, self.state.subscription()])
    }

    pub fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::View(stage) => {
                self.context.set_stage(stage);
                self.state = new_state(&self.context);
                self.state.load(&self.context)
            }
            Message::Sync => self.state.load(&self.context),
            Message::Clipboard(data) => clipboard::write(data),
            _ => self.state.update(&mut self.context, message),
        }
    }

    pub fn view(&self) -> Element<Message> {
        self.state.view(&self.context)
    }
}
