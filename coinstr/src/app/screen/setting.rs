// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{radio, Column};
use iced::{Command, Element};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::Text;
use crate::constants::APP_NAME;
use crate::theme::Theme;

#[derive(Debug, Clone)]
pub enum SettingMessage {
    ThemeChanged(Theme),
}

#[derive(Debug, Default)]
pub struct SettingState {}

impl SettingState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for SettingState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Setting")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Setting(msg) = message {
            match msg {
                SettingMessage::ThemeChanged(theme) => ctx.theme = theme,
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
                    |theme| SettingMessage::ThemeChanged(theme).into(),
                ))
            },
        );
        let content = Column::new().push(choose_theme);
        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<SettingState> for Box<dyn State> {
    fn from(s: SettingState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SettingMessage> for Message {
    fn from(msg: SettingMessage) -> Self {
        Self::Setting(msg)
    }
}
