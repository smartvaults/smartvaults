// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::button;
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum AddSignerMessage {}

#[derive(Debug, Default)]
pub struct AddSignerState {}

impl AddSignerState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddSignerState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Add signer")
    }

    fn update(&mut self, _ctx: &mut Context, _message: Message) -> Command<Message> {
        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = Column::new()
            .push(
                button::primary("Connect Signing Device")
                    .on_press(Message::View(Stage::AddHWSigner))
                    .width(Length::Fill),
            )
            .push(
                button::primary("Add AirGap Signer")
                    .on_press(Message::View(Stage::AddAirGapSigner))
                    .width(Length::Fill),
            )
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<AddSignerState> for Box<dyn State> {
    fn from(s: AddSignerState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddSignerMessage> for Message {
    fn from(msg: AddSignerMessage) -> Self {
        Self::AddSigner(msg)
    }
}
