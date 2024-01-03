// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::net::SocketAddr;

use iced::widget::{Column, Row};
use iced::{Command, Element, Length};
use smartvaults_sdk::config::{Config, ElectrumEndpoint};
use smartvaults_sdk::nostr::Url;

use super::view;
use crate::component::{rule, Button, ButtonStyle, Text, TextInput};
use crate::start::{Context, Message, Stage, State};
use crate::theme::color::DARK_RED;
use crate::BASE_PATH;

#[derive(Debug, Clone)]
pub enum SettingMessage {
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
pub struct SettingState {
    electrum_endpoint: String,
    proxy: String,
    block_explorer: String,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl SettingState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for SettingState {
    fn title(&self) -> String {
        String::from("Settings")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let network = ctx.network;
        Command::perform(
            async move {
                let config = Config::try_from_file(BASE_PATH.as_path(), network)?;
                Ok::<
                    (Option<ElectrumEndpoint>, Option<SocketAddr>, Option<Url>),
                    Box<dyn std::error::Error>,
                >((
                    config.electrum_endpoint().await.ok(),
                    config.proxy().await.ok(),
                    config.block_explorer().await.ok(),
                ))
            },
            |res| match res {
                Ok((electrum, proxy, block_explorer)) => SettingMessage::Load {
                    electrum_endpoint: electrum.map(|e| e.to_string()).unwrap_or_default(),
                    proxy: proxy.map(|p| p.to_string()).unwrap_or_default(),
                    block_explorer: block_explorer.map(|u| u.to_string()).unwrap_or_default(),
                }
                .into(),
                Err(e) => SettingMessage::ErrorChanged(Some(e.to_string())).into(),
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Setting(msg) = message {
            match msg {
                SettingMessage::Load {
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
                SettingMessage::ElectrumEndpointChanged(electrum_endpoint) => {
                    self.electrum_endpoint = electrum_endpoint
                }
                SettingMessage::ProxyChanged(proxy) => self.proxy = proxy,
                SettingMessage::BlockExplorerChanged(block_explorer) => {
                    self.block_explorer = block_explorer
                }
                SettingMessage::ErrorChanged(e) => {
                    self.loading = false;
                    self.error = e;
                }
                SettingMessage::Save => {
                    self.loading = true;
                    let network = ctx.network;
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

                            let config = Config::try_from_file(BASE_PATH.as_path(), network)?;
                            config.set_electrum_endpoint(Some(endpoint)).await?;
                            config.set_proxy(proxy).await;
                            config.set_block_explorer(block_explorer).await;
                            config.save().await?;

                            Ok::<(), Box<dyn std::error::Error>>(())
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Open),
                            Err(e) => SettingMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        };

        Command::none()
    }

    fn view(&self, _ctx: &Context) -> Element<Message> {
        let electrum_endpoint = TextInput::new(&self.electrum_endpoint)
            .label("Electrum Server")
            .on_input(|s| SettingMessage::ElectrumEndpointChanged(s).into())
            .placeholder("Electrum Server")
            .view();

        let proxy = TextInput::new(&self.proxy)
            .label("Proxy")
            .on_input(|s| SettingMessage::ProxyChanged(s).into())
            .placeholder("Proxy")
            .view();

        let block_explorer = TextInput::new(&self.block_explorer)
            .label("Block Explorer")
            .on_input(|s| SettingMessage::BlockExplorerChanged(s).into())
            .placeholder("Block Explorer")
            .view();

        let save_btn = Button::new()
            .text("Save")
            .on_press(SettingMessage::Save.into())
            .loading(self.loading)
            .width(Length::Fill);

        let open_btn = Button::new()
            .text("Open keychain")
            .style(ButtonStyle::Bordered)
            .width(Length::Fill)
            .on_press(Message::View(Stage::Open));

        let restore_keychain_btn = Button::new()
            .text("Restore keychain")
            .style(ButtonStyle::Bordered)
            .on_press(Message::View(Stage::Restore))
            .width(Length::Fill);

        let content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("Settings").big().bold().view())
                    .push(Text::new("Edit settings").extra_light().view())
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
            .push(rule::horizontal())
            .push(open_btn.view())
            .push(restore_keychain_btn.view());

        view(content)
    }
}

impl From<SettingState> for Box<dyn State> {
    fn from(s: SettingState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SettingMessage> for Message {
    fn from(msg: SettingMessage) -> Self {
        Self::Setting(msg)
    }
}
