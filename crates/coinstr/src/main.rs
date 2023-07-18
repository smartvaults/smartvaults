// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

#![allow(clippy::new_without_default)]
#![windows_subsystem = "windows"]

use std::path::PathBuf;
use std::str::FromStr;

use coinstr_sdk::core::bitcoin::Network;
use coinstr_sdk::logger;
use iced::{executor, Application, Command, Element, Settings, Subscription, Theme};
use once_cell::sync::Lazy;

mod app;
mod component;
mod constants;
mod start;
mod theme;

use self::constants::APP_NAME;

static BASE_PATH: Lazy<PathBuf> =
    Lazy::new(|| coinstr_common::base_path().expect("Impossible to get coinstr path"));

fn parse_network(args: Vec<String>) -> Network {
    for (i, arg) in args.iter().enumerate() {
        if arg.contains("--") {
            let network = Network::from_str(args[i].trim_start_matches("--")).unwrap();
            return network;
        }
    }
    Network::Bitcoin
}

pub fn main() -> iced::Result {
    let network = parse_network(std::env::args().collect());
    let mut settings = Settings::with_flags(network);
    settings.id = Some(String::from("io.coinstr.desktop"));
    settings.window.min_size = Some((1000, 700));
    settings.text_multithreading = true;
    settings.antialiasing = false;
    settings.default_font = Some(theme::font::REGULAR_BYTES);

    logger::init(BASE_PATH.clone(), network).unwrap();

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
    type Flags = Network;
    type Message = Message;
    type Theme = Theme;

    fn new(network: Network) -> (Self, Command<Self::Message>) {
        let stage = start::Start::new(network);
        (
            Self {
                state: State::Start(stage.0),
            },
            stage.1.map(|m| m.into()),
        )
    }

    fn title(&self) -> String {
        let (title, network) = match &self.state {
            State::Start(auth) => (auth.title(), auth.ctx.network),
            State::App(app) => (app.title(), app.ctx.client.network()),
        };

        let mut title = if title.is_empty() {
            APP_NAME.to_string()
        } else {
            format!("{APP_NAME} - {title}")
        };

        if network != Network::Bitcoin {
            title.push_str(&format!(" [{network}]"));
        }

        title
    }

    fn theme(&self) -> Theme {
        match &self.state {
            State::Start(start) => start.theme().into(),
            State::App(app) => app.theme().into(),
        }
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
                    return Command::perform(async {}, |_| {
                        Message::App(Box::new(app::Message::Tick))
                    });
                }
                command.map(|m| m.into())
            }
            (State::App(app), Message::App(msg)) => match *msg {
                app::Message::Lock => {
                    let client = app.ctx.client.clone();
                    tokio::task::spawn(async move {
                        if let Err(e) = client.shutdown().await {
                            log::error!("Impossible to shutdown client: {}", e.to_string());
                        }
                    });
                    let new = Self::new(app.ctx.client.network());
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
