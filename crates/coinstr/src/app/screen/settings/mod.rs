// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{radio, Column};
use iced::{Command, Element};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text};
use crate::theme::Theme;

pub mod relays;

#[derive(Debug, Clone)]
pub enum SettingsMessage {
    ThemeChanged(Theme),
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
                SettingsMessage::ThemeChanged(theme) => ctx.theme = theme,
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
        let choose_theme = [Theme::Light, Theme::Dark].iter().fold(
            Column::new()
                .push(Text::new("Choose a theme:").view())
                .spacing(10),
            |column, theme| {
                column.push(radio(
                    format!("{theme}"),
                    *theme,
                    Some(ctx.theme),
                    |theme| SettingsMessage::ThemeChanged(theme).into(),
                ))
            },
        );
        let content = Column::new()
            .push(choose_theme)
            .push(button::primary("Relays").on_press(Message::View(Stage::Relays)))
            .push(
                button::primary("Rebroadcast all events")
                    .on_press(SettingsMessage::RebroadcastAllEvents.into()),
            )
            .push(
                button::danger_border("Clear cache").on_press(SettingsMessage::ClearCache.into()),
            );
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
