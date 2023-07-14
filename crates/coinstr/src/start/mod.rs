// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bitcoin::Network;
use iced::{Command, Element, Subscription};

mod context;
mod message;
pub mod screen;

pub use self::context::{Context, Stage};
pub use self::message::Message;
use self::screen::{GenerateState, OpenState, RestoreState, SettingState};
use crate::app::App;
use crate::theme::Theme;
use crate::CoinstrApp;

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
        // TODO: load theme from config
        let ctx = Context::new(stage, network, Theme::default());
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
        self.ctx.theme
    }

    pub fn subscription(&self) -> Subscription<Message> {
        self.state.subscription()
    }

    pub fn update(&mut self, message: Message) -> (Command<Message>, Option<CoinstrApp>) {
        match message {
            Message::View(stage) => {
                self.ctx.set_stage(stage);
                self.state = new_state(&self.ctx);
                (self.state.load(&self.ctx), None)
            }
            Message::Load => (self.state.load(&self.ctx), None),
            Message::OpenResult(coinstr) => {
                let app = App::new(coinstr, self.ctx.theme);
                (
                    Command::none(),
                    Some(CoinstrApp {
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
