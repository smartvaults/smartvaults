// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_core::bip39::Mnemonic;
use coinstr_core::Coinstr;
use iced::widget::{Checkbox, Column, Row};
use iced::{Command, Element, Length};

use super::view;
use crate::component::{button, rule, Text, TextInput};
use crate::constants::APP_NAME;
use crate::start::{Context, Message, Stage, State};
use crate::theme::color::DARK_RED;
use crate::BASE_PATH;

#[derive(Debug, Clone)]
pub enum RestoreMessage {
    NameChanged(String),
    PasswordChanged(String),
    ConfirmPasswordChanged(String),
    MnemonicChanged(String),
    UsePassphrase(bool),
    PassphraseChanged(String),
    RestoreButtonPressed,
}

#[derive(Debug, Default)]
pub struct RestoreState {
    name: String,
    password: String,
    confirm_password: String,
    mnemonic: String,
    use_passphrase: bool,
    passphrase: String,
    error: Option<String>,
}

impl RestoreState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for RestoreState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Restore")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Restore(msg) = message {
            match msg {
                RestoreMessage::NameChanged(name) => self.name = name,
                RestoreMessage::PasswordChanged(passwd) => self.password = passwd,
                RestoreMessage::ConfirmPasswordChanged(passwd) => self.confirm_password = passwd,
                RestoreMessage::MnemonicChanged(mnemonic) => self.mnemonic = mnemonic,
                RestoreMessage::UsePassphrase(value) => {
                    self.use_passphrase = value;
                    self.passphrase = String::new();
                }
                RestoreMessage::PassphraseChanged(passphrase) => self.passphrase = passphrase,
                RestoreMessage::RestoreButtonPressed => {
                    if self.password.eq(&self.confirm_password) {
                        match Coinstr::restore(
                            BASE_PATH.as_path(),
                            self.name.clone(),
                            || Ok(self.password.clone()),
                            || Ok(Mnemonic::from_str(&self.mnemonic)?),
                            || Ok(Some(self.passphrase.clone())),
                            ctx.network,
                        ) {
                            Ok(keechain) => {
                                return Command::perform(async {}, move |_| {
                                    Message::OpenResult(keechain)
                                })
                            }
                            Err(e) => self.error = Some(e.to_string()),
                        }
                    } else {
                        self.error = Some("Passwords not match".to_string())
                    }
                }
            }
        };

        Command::none()
    }

    fn view(&self, _ctx: &Context) -> Element<Message> {
        let name = TextInput::new("Name", &self.name)
            .on_input(|s| Message::Restore(RestoreMessage::NameChanged(s)))
            .placeholder("Name of keychain")
            .view();

        let password = TextInput::new("Password", &self.password)
            .on_input(|s| Message::Restore(RestoreMessage::PasswordChanged(s)))
            .placeholder("Password")
            .password()
            .view();

        let confirm_password = TextInput::new("Confirm password", &self.confirm_password)
            .on_input(|s| Message::Restore(RestoreMessage::ConfirmPasswordChanged(s)))
            .placeholder("Confirm password")
            .password()
            .view();

        let mnemonic = TextInput::new("Mnemonic (BIP39)", &self.mnemonic)
            .on_input(|s| Message::Restore(RestoreMessage::MnemonicChanged(s)))
            .placeholder("Mnemonic")
            .view();

        let use_passphrase = Checkbox::new("Use a passphrase", self.use_passphrase, |value| {
            RestoreMessage::UsePassphrase(value).into()
        })
        .width(Length::Fill);

        let passphrase = if self.use_passphrase {
            TextInput::new("Passphrase", &self.passphrase)
                .on_input(|s| Message::Restore(RestoreMessage::PassphraseChanged(s)))
                .placeholder("Passphrase")
                .view()
        } else {
            Column::new()
        };

        let restore_keychain_btn = button::primary("Restore")
            .on_press(Message::Restore(RestoreMessage::RestoreButtonPressed))
            .width(Length::Fill);

        let open_btn = button::border("Open keychain")
            .width(Length::Fill)
            .on_press(Message::View(Stage::Open));

        let new_keychain_btn = button::border("Create keychain")
            .on_press(Message::View(Stage::New))
            .width(Length::Fill);

        let content = Column::new()
            .push(name)
            .push(password)
            .push(confirm_password)
            .push(mnemonic)
            .push(use_passphrase)
            .push(passphrase)
            .push(if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            })
            .push(restore_keychain_btn)
            .push(rule::horizontal())
            .push(open_btn)
            .push(new_keychain_btn);

        view(content)
    }
}

impl From<RestoreState> for Box<dyn State> {
    fn from(s: RestoreState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<RestoreMessage> for Message {
    fn from(msg: RestoreMessage) -> Self {
        Self::Restore(msg)
    }
}
