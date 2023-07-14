// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Command, Element};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, ButtonStyle};

pub mod add_relay;
pub mod config;
pub mod relays;

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    RebroadcastAllEvents,
    ClearCache,
}

#[derive(Debug, Default)]
pub struct SettingsState {}

impl SettingsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for SettingsState {
    fn title(&self) -> String {
        String::from("Settings")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Settings(msg) = message {
            match msg {
                SettingsMessage::RebroadcastAllEvents => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.rebroadcast_all_events().await.unwrap() },
                        move |_| Message::View(Stage::Dashboard),
                    );
                }
                SettingsMessage::ClearCache => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.clear_cache().await.unwrap() },
                        move |_| Message::View(Stage::Dashboard),
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = Column::new()
            .push(
                Button::new()
                    .text("Config")
                    .on_press(Message::View(Stage::Config))
                    .view(),
            )
            .push(
                Button::new()
                    .text("Relays")
                    .on_press(Message::View(Stage::Relays))
                    .view(),
            )
            .push(
                Button::new()
                    .text("Rebroadcast all events")
                    .on_press(SettingsMessage::RebroadcastAllEvents.into())
                    .view(),
            )
            .push(
                Button::new()
                    .text("Clear cache")
                    .style(ButtonStyle::BorderedDanger)
                    .on_press(SettingsMessage::ClearCache.into())
                    .view(),
            )
            .spacing(10);
        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<SettingsState> for Box<dyn State> {
    fn from(s: SettingsState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SettingsMessage> for Message {
    fn from(msg: SettingsMessage) -> Self {
        Self::Settings(msg)
    }
}
