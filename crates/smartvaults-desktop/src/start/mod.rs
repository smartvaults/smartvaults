// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::{Command, Element, Subscription};
use smartvaults_sdk::core::bitcoin::Network;

mod context;
mod message;
pub mod screen;

pub use self::context::{Context, Stage};
pub use self::message::Message;
use self::screen::{GenerateState, OpenState, RestoreState, SettingState};
use crate::app::App;
use crate::theme::Theme;
use crate::SmartVaultsApp;

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

pub fn new_state(context: &Context) -> Box<dyn State> {
    match &context.stage {
        Stage::Open => OpenState::new().into(),
        Stage::New => GenerateState::new().into(),
        Stage::Restore => RestoreState::new().into(),
        Stage::Setting => SettingState::new().into(),
    }
}

pub struct Start {
    state: Box<dyn State>,
    pub(crate) ctx: Context,
}

impl Start {
    pub fn new(network: Network) -> (Self, Command<Message>) {
        let stage = Stage::default();
        let ctx = Context::new(stage, network);
        let app = Self {
            state: new_state(&ctx),
            ctx,
        };
        (app, Command::perform(async {}, move |_| Message::Load))
    }

    pub fn title(&self) -> String {
        self.state.title()
    }

    pub fn theme(&self) -> Theme {
        match self.ctx.network {
            Network::Bitcoin => Theme::Mainnet,
            Network::Testnet => Theme::Testnet,
            Network::Signet => Theme::Signet,
            _ => Theme::Regtest,
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.state.subscription()
    }

    pub fn update(&mut self, message: Message) -> (Command<Message>, Option<SmartVaultsApp>) {
        match message {
            Message::View(stage) => {
                self.ctx.set_stage(stage);
                self.state = new_state(&self.ctx);
                (self.state.load(&self.ctx), None)
            }
            Message::Load => (self.state.load(&self.ctx), None),
            Message::OpenResult(client) => {
                let app = App::new(client);
                (
                    Command::none(),
                    Some(SmartVaultsApp {
                        state: crate::State::App(app),
                    }),
                )
            }
            _ => (self.state.update(&mut self.ctx, message), None),
        }
    }

    pub fn view(&self) -> Element<Message> {
        self.state.view(&self.ctx)
    }
}
