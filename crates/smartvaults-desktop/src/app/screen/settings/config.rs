// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::net::SocketAddr;

use iced::widget::{Column, Row};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::nostr::Url;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum ConfigMessage {
    Load {
        electrum_endpoint: String,
        proxy: String,
        block_explorer: String,
    },
    ElectrumEndpointChanged(String),
    ProxyChanged(String),
    BlockExplorerChanged(String),
    ErrorChanged(Option<String>),
    Save,
}

#[derive(Debug, Default)]
pub struct ConfigState {
    electrum_endpoint: String,
    proxy: String,
    block_explorer: String,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl ConfigState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ConfigState {
    fn title(&self) -> String {
        String::from("Configs")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let config = ctx.client.config();
        Command::perform(
            async move {
                (
                    config.electrum_endpoint().await.ok(),
                    config.proxy().await.ok(),
                    config.block_explorer().await.ok(),
                )
            },
            |(electrum, proxy, block_explorer)| {
                ConfigMessage::Load {
                    electrum_endpoint: electrum.map(|e| e.to_string()).unwrap_or_default(),
                    proxy: proxy.map(|p| p.to_string()).unwrap_or_default(),
                    block_explorer: block_explorer.map(|u| u.to_string()).unwrap_or_default(),
                }
                .into()
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Config(msg) = message {
            match msg {
                ConfigMessage::Load {
                    electrum_endpoint,
                    proxy,
                    block_explorer,
                } => {
                    self.electrum_endpoint = electrum_endpoint;
                    self.proxy = proxy;
                    self.block_explorer = block_explorer;
                    self.loaded = true;
                    self.loading = false;
                }
                ConfigMessage::ElectrumEndpointChanged(electrum_endpoint) => {
                    self.electrum_endpoint = electrum_endpoint
                }
                ConfigMessage::ProxyChanged(proxy) => self.proxy = proxy,
                ConfigMessage::BlockExplorerChanged(block_explorer) => {
                    self.block_explorer = block_explorer
                }
                ConfigMessage::ErrorChanged(e) => {
                    self.loading = false;
                    self.error = e;
                }
                ConfigMessage::Save => {
                    self.loading = true;
                    let config = ctx.client.config();
                    let endpoint = self.electrum_endpoint.clone();
                    let proxy = self.proxy.clone();
                    let block_explorer = self.block_explorer.clone();

                    return Command::perform(
                        async move {
                            let proxy: Option<SocketAddr> = if proxy.is_empty() {
                                None
                            } else {
                                Some(proxy.parse::<SocketAddr>()?)
                            };

                            let block_explorer: Option<Url> = if block_explorer.is_empty() {
                                None
                            } else {
                                Some(Url::parse(&block_explorer)?)
                            };

                            config.set_electrum_endpoint(Some(endpoint)).await?;
                            config.set_proxy(proxy).await;
                            config.set_block_explorer(block_explorer).await;
                            config.save().await?;

                            Ok::<(), Box<dyn std::error::Error>>(())
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Settings),
                            Err(e) => ConfigMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        };

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let electrum_endpoint = TextInput::with_label("Electrum Server", &self.electrum_endpoint)
            .on_input(|s| ConfigMessage::ElectrumEndpointChanged(s).into())
            .placeholder("Electrum Server")
            .view();

        let proxy = TextInput::with_label("Proxy", &self.proxy)
            .on_input(|s| ConfigMessage::ProxyChanged(s).into())
            .placeholder("Proxy")
            .view();

        let block_explorer = TextInput::with_label("Block Explorer", &self.block_explorer)
            .on_input(|s| ConfigMessage::BlockExplorerChanged(s).into())
            .placeholder("Block Explorer")
            .view();

        let save_btn = Button::new()
            .text("Save")
            .on_press(ConfigMessage::Save.into())
            .loading(self.loading)
            .width(Length::Fill);

        let content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("Configs").big().bold().view())
                    .push(Text::new("Edit Configs").extra_light().view())
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(electrum_endpoint)
            .push(proxy)
            .push(block_explorer)
            .push(if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            })
            .push(save_btn.view())
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
    }
}

impl From<ConfigState> for Box<dyn State> {
    fn from(s: ConfigState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ConfigMessage> for Message {
    fn from(msg: ConfigMessage) -> Self {
        Self::Config(msg)
    }
}
