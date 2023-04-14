// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use std::path::PathBuf;

use iced::{executor, Application, Command, Element, Settings, Subscription, Theme};
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

mod app;
mod component;
mod constants;
mod start;
mod theme;

static KEYCHAINS_PATH: Lazy<PathBuf> =
    Lazy::new(|| coinstr_common::keychains().expect("Impossible to get keychains path"));
static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().expect("Can't start Tokio runtime"));

pub fn main() -> iced::Result {
    env_logger::init();

    let mut settings = Settings::default();
    settings.window.min_size = Some((600, 600));
    settings.default_font = Some(theme::font::REGULAR_BYTES);
    CoinstrApp::run(settings)
}

pub struct CoinstrApp {
    state: State,
}
pub enum State {
    Start(start::Start),
    App(app::App),
}

#[derive(Debug, Clone)]
pub enum Message {
    Start(Box<start::Message>),
    App(Box<app::Message>),
}

impl Application for CoinstrApp {
    type Executor = executor::Default;
    type Flags = ();
    type Message = Message;
    type Theme = Theme;

    fn new(_flags: ()) -> (Self, Command<Self::Message>) {
        let stage = start::Start::new();
        (
            Self {
                state: State::Start(stage.0),
            },
            stage.1.map(|m| m.into()),
        )
    }

    fn title(&self) -> String {
        match &self.state {
            State::Start(auth) => auth.title(),
            State::App(app) => app.title(),
        }
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        match &self.state {
            State::Start(start) => start.subscription().map(|m| m.into()),
            State::App(app) => app.subscription().map(|m| m.into()),
        }
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match (&mut self.state, message) {
            (State::Start(start), Message::Start(msg)) => {
                let (command, stage_to_move) = start.update(*msg);
                if let Some(stage) = stage_to_move {
                    *self = stage;
                }
                command.map(|m| m.into())
            }
            (State::App(app), Message::App(msg)) => match *msg {
                app::Message::Lock => {
                    let client = app.context.client.inner();
                    tokio::task::spawn(async move {
                        if let Err(e) = client.shutdown().await {
                            log::error!("Impossible to shutdown client: {}", e.to_string());
                        }
                    });
                    let new = Self::new(());
                    *self = new.0;
                    new.1
                }
                _ => app.update(*msg).map(|m| m.into()),
            },
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<Self::Message> {
        match &self.state {
            State::Start(start) => start.view().map(|m| m.into()),
            State::App(app) => app.view().map(|m| m.into()),
        }
    }
}
