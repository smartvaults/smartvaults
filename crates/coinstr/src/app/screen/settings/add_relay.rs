// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::net::SocketAddr;

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum AddRelayMessage {
    RelayUrlChanged(String),
    ProxyChanged(String),
    ProxyToggled(bool),
    ErrorChanged(Option<String>),
    AddRelay,
}

#[derive(Debug, Default)]
pub struct AddRelayState {
    url: String,
    proxy: String,
    use_proxy: bool,
    loading: bool,
    error: Option<String>,
}

impl AddRelayState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddRelayState {
    fn title(&self) -> String {
        String::from("Add relay")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::AddRelay(msg) = message {
            match msg {
                AddRelayMessage::RelayUrlChanged(url) => self.url = url,
                AddRelayMessage::ProxyChanged(proxy) => self.proxy = proxy,
                AddRelayMessage::ProxyToggled(value) => self.use_proxy = value,
                AddRelayMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                AddRelayMessage::AddRelay => {
                    self.loading = true;
                    let url = self.url.clone();
                    let use_proxy = self.use_proxy;
                    let proxy = self.proxy.clone();
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move {
                            let proxy: Option<SocketAddr> = if use_proxy {
                                Some(proxy.parse()?)
                            } else {
                                None
                            };

                            client.add_relay(url, proxy).await?;
                            client.connect().await;

                            Ok::<(), Box<dyn std::error::Error>>(())
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Relays),
                            Err(e) => AddRelayMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let url = TextInput::new("Relay url", &self.url)
            .on_input(|s| AddRelayMessage::RelayUrlChanged(s).into())
            .placeholder("Relay url")
            .view();

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let content = Column::new()
            .push(url)
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(
                Button::new()
                    .text("Add relay")
                    .on_press(AddRelayMessage::AddRelay.into())
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

impl From<AddRelayState> for Box<dyn State> {
    fn from(s: AddRelayState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddRelayMessage> for Message {
    fn from(msg: AddRelayMessage) -> Self {
        Self::AddRelay(msg)
    }
}
