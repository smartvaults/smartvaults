// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::path::PathBuf;
use std::str::FromStr;

use coinstr_core::bitcoin::Network;
use iced::{executor, Application, Command, Element, Settings, Subscription, Theme as NativeTheme};
use once_cell::sync::Lazy;
use tokio::runtime::Runtime;

mod app;
mod component;
mod constants;
mod start;
mod theme;

use self::theme::Theme;

static KEYCHAINS_PATH: Lazy<PathBuf> =
    Lazy::new(|| coinstr_common::keychains().expect("Impossible to get keychains path"));
static RUNTIME: Lazy<Runtime> = Lazy::new(|| Runtime::new().expect("Can't start Tokio runtime"));

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
    env_logger::init();

    let network = parse_network(std::env::args().collect());
    let mut settings = Settings::with_flags(network);
    settings.window.min_size = Some((600, 600));
    settings.text_multithreading = true;
    settings.default_font = Some(theme::font::REGULAR_BYTES);
    CoinstrApp::run(settings)
}

pub struct CoinstrApp {
    state: State,
    theme: Theme,
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
    type Theme = NativeTheme;

    fn new(network: Network) -> (Self, Command<Self::Message>) {
        let stage = start::Start::new(network);
        (
            Self {
                state: State::Start(stage.0),
                theme: Theme::default(),
            },
            stage.1.map(|m| m.into()),
        )
    }

    fn title(&self) -> String {
        let (title, network) = match &self.state {
            State::Start(auth) => (auth.title(), auth.context.network),
            State::App(app) => (app.title(), app.context.coinstr.network()),
        };

        if network == Network::Bitcoin {
            title
        } else {
            format!("{title} [{network}]")
        }
    }

    fn theme(&self) -> NativeTheme {
        self.theme.into()
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
                    let new = Self::new(app.context.coinstr.network());
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
