// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

#![allow(clippy::new_without_default)]
#![windows_subsystem = "windows"]

use std::path::PathBuf;
use std::str::FromStr;

use constants::DEFAULT_FONT_SIZE;
use iced::window::Event as WindowEvent;
use iced::{
    executor, font, subscription, Application, Command, Element, Event, Settings, Subscription,
    Theme,
};
use once_cell::sync::Lazy;
use smartvaults_sdk::core::bitcoin::Network;
use smartvaults_sdk::core::Result;
use smartvaults_sdk::logger;
use theme::font::{
    BOOTSTRAP_ICONS_BYTES, REGULAR, ROBOTO_MONO_BOLD_BYTES, ROBOTO_MONO_LIGHT_BYTES,
    ROBOTO_MONO_REGULAR_BYTES,
};

mod app;
mod component;
mod constants;
mod start;
mod theme;

use self::constants::APP_NAME;

fn base_path() -> Result<PathBuf> {
    let home_path = dirs::home_dir().expect("Imposible to get the HOME dir");
    let old_path = home_path.join(".coinstr");
    let path = home_path.join(".smartvaults");
    if old_path.exists() && !path.exists() {
        std::fs::rename(old_path, &path).unwrap();
    }
    std::fs::create_dir_all(path.as_path())?;
    Ok(path)
}

static BASE_PATH: Lazy<PathBuf> = Lazy::new(|| base_path().expect("Impossible to get base path"));

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
    settings.id = Some(String::from("app.smartvaults.desktop"));
    settings.window.min_size = Some((1000, 700));
    settings.exit_on_close_request = false;
    settings.antialiasing = false;
    settings.default_text_size = DEFAULT_FONT_SIZE as f32;
    settings.default_font = REGULAR;

    logger::init(BASE_PATH.clone(), network, true).unwrap();

    SmartVaultsApp::run(settings)
}

pub struct SmartVaultsApp {
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
    FontLoaded(Result<(), font::Error>),
    EventOccurred(Event),
}

impl Application for SmartVaultsApp {
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
            Command::batch(vec![
                font::load(ROBOTO_MONO_REGULAR_BYTES).map(Message::FontLoaded),
                font::load(ROBOTO_MONO_LIGHT_BYTES).map(Message::FontLoaded),
                font::load(ROBOTO_MONO_BOLD_BYTES).map(Message::FontLoaded),
                font::load(BOOTSTRAP_ICONS_BYTES).map(Message::FontLoaded),
                stage.1.map(|m| m.into()),
            ]),
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
        let stage_sub = match &self.state {
            State::Start(start) => start.subscription().map(|m| m.into()),
            State::App(app) => app.subscription().map(|m| m.into()),
        };
        Subscription::batch(vec![
            subscription::events().map(Message::EventOccurred),
            stage_sub,
        ])
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
                            tracing::error!("Impossible to shutdown client: {}", e.to_string());
                        }
                    });
                    let new = Self::new(app.ctx.client.network());
                    *self = new.0;
                    new.1
                }
                _ => app.update(*msg).map(|m| m.into()),
            },
            (_, Message::EventOccurred(Event::Window(WindowEvent::CloseRequested))) => {
                tracing::debug!("Pressed close button");
                std::process::exit(0x00)
            }
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
