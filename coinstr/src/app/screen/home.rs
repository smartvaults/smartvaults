// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Command, Element};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};

#[derive(Debug, Clone)]
pub enum HomeMessage {}

#[derive(Debug, Default)]
pub struct HomeState {}

impl HomeState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for HomeState {
    fn title(&self) -> String {
        String::from("KeeChain - Home")
    }

    fn update(&mut self, _ctx: &mut Context, _message: Message) -> Command<Message> {
        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        Dashboard::new().view(ctx, Column::new())
    }
}

impl From<HomeState> for Box<dyn State> {
    fn from(s: HomeState) -> Box<dyn State> {
        Box::new(s)
    }
}
