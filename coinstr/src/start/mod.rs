// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::{Command, Element, Subscription};

mod context;
mod message;
pub mod screen;

pub use self::context::{Context, Stage};
pub use self::message::Message;
use self::screen::{OpenState, RestoreState};
use crate::app::App;
use crate::constants::APP_NAME;
use crate::theme::Theme;
use crate::CoinstrApp;

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

pub fn new_state(context: &Context) -> Box<dyn State> {
    match &context.stage {
        Stage::Open => OpenState::new().into(),
        Stage::New => todo!(),
        Stage::Restore => RestoreState::new().into(),
    }
}

pub struct Start {
    state: Box<dyn State>,
    context: Context,
}

impl Start {
    pub fn new() -> (Self, Command<Message>) {
        let stage = Stage::default();
        let context = Context::new(stage);
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
        self.state.subscription()
    }

    pub fn update(&mut self, message: Message) -> (Command<Message>, Option<CoinstrApp>) {
        match message {
            Message::View(stage) => {
                self.context.set_stage(stage);
                self.state = new_state(&self.context);
                (self.state.load(&self.context), None)
            }
            Message::OpenResult(coinstr) => {
                let (app, _) = App::new(coinstr);
                (
                    Command::none(),
                    Some(CoinstrApp {
                        state: crate::State::App(app),
                        theme: Theme::default(),
                    }),
                )
            }
            _ => (self.state.update(&mut self.context, message), None),
        }
    }

    pub fn view(&self) -> Element<Message> {
        self.state.view(&self.context)
    }
}
