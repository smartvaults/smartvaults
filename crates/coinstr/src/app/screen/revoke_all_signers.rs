// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, TextInput};
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum RevokeAllSignersMessage {
    ConfirmChanged(String),
    Revoke,
    ErrorChanged(Option<String>),
}

#[derive(Debug, Default)]
pub struct RevokeAllSignersState {
    confirm: String,
    error: Option<String>,
}

impl RevokeAllSignersState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for RevokeAllSignersState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Revoke all shared signers")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::RevokeAllSigners(msg) = message {
            match msg {
                RevokeAllSignersMessage::ConfirmChanged(s) => self.confirm = s,
                RevokeAllSignersMessage::Revoke => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.revoke_all_shared_signers().await },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Signers),
                            Err(e) => {
                                RevokeAllSignersMessage::ErrorChanged(Some(e.to_string())).into()
                            }
                        },
                    );
                }
                RevokeAllSignersMessage::ErrorChanged(error) => self.error = error,
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let confirm = TextInput::new("To confirm, type 'CONFIRM'", &self.confirm)
            .placeholder("To confirm, type 'CONFIRM'")
            .on_input(|s| RevokeAllSignersMessage::ConfirmChanged(s).into())
            .view();
        let mut revoke_all_btn =
            button::danger_border("Revoke all shared signers").width(Length::Fill);

        if self.confirm == *"CONFIRM" {
            revoke_all_btn = revoke_all_btn.on_press(RevokeAllSignersMessage::Revoke.into());
        }

        let content = Column::new()
            .push(confirm)
            .push(revoke_all_btn)
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<RevokeAllSignersState> for Box<dyn State> {
    fn from(s: RevokeAllSignersState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<RevokeAllSignersMessage> for Message {
    fn from(msg: RevokeAllSignersMessage) -> Self {
        Self::RevokeAllSigners(msg)
    }
}
