// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Command, Element};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum SettingMessage {}

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

    fn update(&mut self, _ctx: &mut Context, _message: Message) -> Command<Message> {
        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        Dashboard::new().view(ctx, Column::new(), true, true)
    }
}

impl From<SettingState> for Box<dyn State> {
    fn from(s: SettingState) -> Box<dyn State> {
        Box::new(s)
    }
}
