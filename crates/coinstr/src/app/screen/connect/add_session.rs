// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_sdk::nostr::prelude::NostrConnectURI;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum AddNostrConnectSessionMessage {
    URIChanged(String),
    ErrorChanged(Option<String>),
    Connect,
}

#[derive(Debug, Default)]
pub struct AddNostrConnectSessionState {
    uri: String,
    loading: bool,
    error: Option<String>,
}

impl AddNostrConnectSessionState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddNostrConnectSessionState {
    fn title(&self) -> String {
        String::from("Add session")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::AddNostrConnectSession(msg) = message {
            match msg {
                AddNostrConnectSessionMessage::URIChanged(uri) => self.uri = uri,
                AddNostrConnectSessionMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                AddNostrConnectSessionMessage::Connect => {
                    let client = ctx.client.clone();
                    match NostrConnectURI::from_str(&self.uri) {
                        Ok(uri) => {
                            self.loading = true;
                            return Command::perform(
                                async move { client.new_nostr_connect_session(uri).await },
                                |res| match res {
                                    Ok(_) => Message::View(Stage::NostrConnect),
                                    Err(e) => AddNostrConnectSessionMessage::ErrorChanged(Some(
                                        e.to_string(),
                                    ))
                                    .into(),
                                },
                            );
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let uri = TextInput::new("Nostr Connect URI", &self.uri)
            .on_input(|s| AddNostrConnectSessionMessage::URIChanged(s).into())
            .placeholder("Nostr Connect URI")
            .view();

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("New session").size(24).bold().view())
                    .push(Text::new("Add a new session").extra_light().view())
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(uri)
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(
                Button::new()
                    .text("Connect")
                    .on_press(AddNostrConnectSessionMessage::Connect.into())
                    .loading(self.loading)
                    .width(Length::Fill)
                    .view(),
            )
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<AddNostrConnectSessionState> for Box<dyn State> {
    fn from(s: AddNostrConnectSessionState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddNostrConnectSessionMessage> for Message {
    fn from(msg: AddNostrConnectSessionMessage) -> Self {
        Self::AddNostrConnectSession(msg)
    }
}
