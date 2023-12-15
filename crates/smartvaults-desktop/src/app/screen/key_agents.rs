// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::secp256k1::XOnlyPublicKey;
use smartvaults_sdk::types::KeyAgent;
use smartvaults_sdk::util::{self, format};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{rule, Button, ButtonStyle, Icon, Text};
use crate::theme::color::{GREEN, RED};
use crate::theme::icon::{CLIPBOARD, FULLSCREEN, PATCH_CHECK, PATCH_EXCLAMATION, PLUS, RELOAD};

#[derive(Debug, Clone)]
pub enum KeyAgentsMessage {
    Load(Vec<KeyAgent>),
    Request(XOnlyPublicKey),
    ErrorChanged(Option<String>),
    Reload,
}

#[derive(Debug, Default)]
pub struct KeyAgentsState {
    loading: bool,
    loaded: bool,
    key_agents: Vec<KeyAgent>,
    error: Option<String>,
}

impl KeyAgentsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for KeyAgentsState {
    fn title(&self) -> String {
        String::from("Key Agents")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(async move { client.key_agents().await.unwrap() }, |p| {
            KeyAgentsMessage::Load(p).into()
        })
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::KeyAgents(msg) = message {
            match msg {
                KeyAgentsMessage::Load(key_agents) => {
                    self.key_agents = key_agents;
                    self.loading = false;
                    self.loaded = true;
                }
                KeyAgentsMessage::Request(public_key) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.request_signers_to_key_agent(public_key).await },
                        |res| match res {
                            Ok(_) => KeyAgentsMessage::Reload.into(),
                            Err(e) => KeyAgentsMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                KeyAgentsMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                KeyAgentsMessage::Reload => {
                    self.loading = false;
                    return self.load(ctx);
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            if self.key_agents.is_empty() {
                content = content
                    .push(Text::new("No key agents found").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .icon(RELOAD)
                            .text("Reload")
                            .width(Length::Fixed(250.0))
                            .on_press(KeyAgentsMessage::Reload.into())
                            .view(),
                    )
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                content = content
                    .push(
                        Row::new()
                            .push(
                                Column::new()
                                    .push(Text::new("Verified").bold().big().view())
                                    .width(Length::Fixed(120.0))
                                    .align_items(Alignment::Center),
                            )
                            .push(
                                Text::new("Public Key")
                                    .bold()
                                    .big()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(Text::new("Name").bold().big().width(Length::Fill).view())
                            .push(Text::new("Website").bold().big().width(Length::Fill).view())
                            .push(
                                Text::new("Offerings")
                                    .bold()
                                    .big()
                                    .width(Length::Fixed(120.0))
                                    .view(),
                            )
                            .push(Space::with_width(Length::Fixed(40.0)))
                            .push(Space::with_width(Length::Fixed(40.0)))
                            .push(
                                Button::new()
                                    .style(ButtonStyle::Bordered)
                                    .icon(RELOAD)
                                    .width(Length::Fixed(40.0))
                                    .on_press(KeyAgentsMessage::Reload.into())
                                    .loading(self.loading)
                                    .view(),
                            )
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for key_agent in self.key_agents.iter() {
                    let public_key = key_agent.public_key();
                    let metadata = key_agent.metadata();

                    let row = Row::new()
                        .push(
                            Column::new()
                                .push(
                                    Icon::new(if key_agent.verified {
                                        PATCH_CHECK
                                    } else {
                                        PATCH_EXCLAMATION
                                    })
                                    .color(if key_agent.verified { GREEN } else { RED })
                                    .big(),
                                )
                                .width(Length::Fixed(120.0))
                                .align_items(Alignment::Center),
                        )
                        .push(
                            Text::new(util::cut_public_key(public_key))
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(Text::new(key_agent.name()).width(Length::Fill).view())
                        .push(
                            Text::new(metadata.website.as_deref().unwrap_or("-"))
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(format::number(key_agent.list.len() as u64))
                                .width(Length::Fixed(120.0))
                                .view(),
                        )
                        .push(
                            Button::new()
                                .style(ButtonStyle::Bordered)
                                .icon(CLIPBOARD)
                                .on_press(Message::Clipboard(public_key.to_string()))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .push(
                            Button::new()
                                .style(ButtonStyle::Bordered)
                                .icon(PLUS)
                                .width(Length::Fixed(40.0))
                                .on_press(KeyAgentsMessage::Request(public_key).into())
                                .loading(self.loading || key_agent.is_contact)
                                .view(),
                        )
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .width(Length::Fixed(40.0))
                                .loading(self.loading)
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill);
                    content = content.push(row).push(rule::horizontal());
                }
            }
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
    }
}

impl From<KeyAgentsState> for Box<dyn State> {
    fn from(s: KeyAgentsState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<KeyAgentsMessage> for Message {
    fn from(msg: KeyAgentsMessage) -> Self {
        Self::KeyAgents(msg)
    }
}
