// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Command, Element};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::Text;
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum DashboardMessage {}

#[derive(Debug, Default)]
pub struct DashboardState {}

impl DashboardState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for DashboardState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Dashboard")
    }

    fn update(&mut self, _ctx: &mut Context, _message: Message) -> Command<Message> {
        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = Column::new()
            .push(Text::new("TODO:").view())
            .push(Text::new("- Total balance").view())
            .push(Text::new("- Send and receive buttons").view())
            .push(Text::new("- All transactions").view());
        Dashboard::new().view(ctx, content, true)
    }
}

impl From<DashboardState> for Box<dyn State> {
    fn from(s: DashboardState) -> Box<dyn State> {
        Box::new(s)
    }
}
