// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, ButtonStyle, TextInput};

#[derive(Debug, Clone)]
pub enum RevokeAllSignersMessage {
    ConfirmChanged(String),
    Revoke,
    ErrorChanged(Option<String>),
}

#[derive(Debug, Default)]
pub struct RevokeAllSignersState {
    confirm: String,
    loading: bool,
    error: Option<String>,
}

impl RevokeAllSignersState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for RevokeAllSignersState {
    fn title(&self) -> String {
        String::from("Revoke all shared signers")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::RevokeAllSigners(msg) = message {
            match msg {
                RevokeAllSignersMessage::ConfirmChanged(s) => self.confirm = s,
                RevokeAllSignersMessage::Revoke => {
                    self.loading = true;
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
                RevokeAllSignersMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let confirm = TextInput::with_label("To confirm, type 'CONFIRM'", &self.confirm)
            .placeholder("To confirm, type 'CONFIRM'")
            .on_input(|s| RevokeAllSignersMessage::ConfirmChanged(s).into())
            .view();

        let content = Column::new()
            .push(confirm)
            .push(
                Button::new()
                    .style(ButtonStyle::BorderedDanger)
                    .text("Revoke all shared signers")
                    .width(Length::Fill)
                    .on_press(RevokeAllSignersMessage::Revoke.into())
                    .loading(self.confirm != *"CONFIRM" || self.loading)
                    .view(),
            )
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
