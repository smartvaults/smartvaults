// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::types::Secrets;
use smartvaults_sdk::core::SECP256K1;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{Button, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum RecoveryKeysMessage {
    PasswordChanged(String),
    Confirm,
    LoadSecrets(Secrets),
    ErrorChanged(Option<String>),
    Null,
}

#[derive(Debug, Default)]
pub struct RecoveryKeysState {
    secrets: Option<Secrets>,
    password: String,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl RecoveryKeysState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for RecoveryKeysState {
    fn title(&self) -> String {
        String::from("Recovery Keys")
    }

    fn load(&mut self, _ctx: &Context) -> Command<Message> {
        self.loaded = true;
        Command::none()
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::RecoveryKeys(msg) = message {
            match msg {
                RecoveryKeysMessage::PasswordChanged(password) => self.password = password,
                RecoveryKeysMessage::ErrorChanged(e) => {
                    self.loading = false;
                    self.error = e;
                }
                RecoveryKeysMessage::Confirm => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let password = self.password.clone();
                    return Command::perform(
                        async move {
                            let keychain = client.keychain(password)?;
                            let secrets = keychain.secrets(client.network(), &SECP256K1)?;
                            Ok::<Secrets, Box<dyn std::error::Error>>(secrets)
                        },
                        |res| match res {
                            Ok(secrets) => RecoveryKeysMessage::LoadSecrets(secrets).into(),
                            Err(e) => RecoveryKeysMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                RecoveryKeysMessage::LoadSecrets(secrets) => {
                    self.password.clear();
                    self.secrets = Some(secrets);
                    self.loading = false;
                }
                RecoveryKeysMessage::Null => (),
            }
        };

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new()
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20);
        let mut center = true;

        if let Some(secrets) = self.secrets.clone() {
            center = false;

            content = content
                .push(
                    TextInput::with_label(
                        format!("Entorpy ({} bits)", secrets.entropy.len() / 2 * 8),
                        secrets.entropy,
                    )
                    .on_input(|_| RecoveryKeysMessage::Null.into())
                    .view(),
                )
                .push(
                    TextInput::with_label("Mnemonic (BIP39)", &secrets.mnemonic.to_string())
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                );

            if let Some(passphrase) = secrets.passphrase {
                content = content.push(
                    TextInput::with_label("Passphrase", &passphrase)
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                );
            }

            content = content
                .push(
                    TextInput::with_label("Seed HEX", &secrets.seed_hex)
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                )
                .push(
                    TextInput::with_label("Root Key (BIP32)", &secrets.root_key.to_string())
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                )
                .push(
                    TextInput::with_label("Fingerprint (BIP32)", &secrets.fingerprint.to_string())
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                );
        } else {
            content = content
                .push(
                    TextInput::with_label("Password", &self.password)
                        .placeholder("Password")
                        .on_input(|p| RecoveryKeysMessage::PasswordChanged(p).into())
                        .on_submit(RecoveryKeysMessage::Confirm.into())
                        .password()
                        .view(),
                )
                .push(if let Some(error) = &self.error {
                    Row::new().push(Text::new(error).color(DARK_RED).view())
                } else {
                    Row::new()
                })
                .push(
                    Button::new()
                        .text("Confirm")
                        .on_press(RecoveryKeysMessage::Confirm.into())
                        .width(Length::Fill)
                        .view(),
                )
                .max_width(400.0)
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, center, center)
    }
}

impl From<RecoveryKeysState> for Box<dyn State> {
    fn from(s: RecoveryKeysState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<RecoveryKeysMessage> for Message {
    fn from(msg: RecoveryKeysMessage) -> Self {
        Self::RecoveryKeys(msg)
    }
}
