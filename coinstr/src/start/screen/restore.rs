// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_core::bip39::Mnemonic;
use coinstr_core::bitcoin::Network;
use coinstr_core::util::dir;
use coinstr_core::Coinstr;
use iced::widget::{Column, Row, Rule};
use iced::{Command, Element, Length};

use super::view;
use crate::component::{button, Text, TextInput};
use crate::start::{Context, Message, Stage, State};
use crate::theme::color::DARK_RED;
use crate::{APP_NAME, KEYCHAINS_PATH};

#[derive(Debug, Clone)]
pub enum RestoreMessage {
    NameChanged(String),
    PasswordChanged(String),
    ConfirmPasswordChanged(String),
    MnemonicChanged(String),
    PassphraseChanged(String),
    RestoreButtonPressed,
}

#[derive(Debug, Default)]
pub struct RestoreState {
    name: String,
    password: String,
    confirm_password: String,
    mnemonic: String,
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
        format!("{APP_NAME} - restore")
    }

    fn update(&mut self, _ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Restore(msg) = message {
            match msg {
                RestoreMessage::NameChanged(name) => self.name = name,
                RestoreMessage::PasswordChanged(passwd) => self.password = passwd,
                RestoreMessage::ConfirmPasswordChanged(passwd) => self.confirm_password = passwd,
                RestoreMessage::MnemonicChanged(mnemonic) => self.mnemonic = mnemonic,
                RestoreMessage::PassphraseChanged(passphrase) => self.passphrase = passphrase,
                RestoreMessage::RestoreButtonPressed => {
                    if self.password.eq(&self.confirm_password) {
                        // TODO: replace network
                        match dir::get_keychain_file(KEYCHAINS_PATH.as_path(), self.name.clone()) {
                            Ok(path) => match Coinstr::restore(
                                path,
                                || Ok(self.password.clone()),
                                || Ok(Mnemonic::from_str(&self.mnemonic)?),
                                || Ok(Some(self.passphrase.clone())),
                                Network::Testnet,
                            ) {
                                Ok(keechain) => {
                                    return Command::perform(async {}, move |_| {
                                        Message::OpenResult(keechain)
                                    })
                                }
                                Err(e) => self.error = Some(e.to_string()),
                            },
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
        let name = TextInput::new("Name", &self.name, |s| {
            Message::Restore(RestoreMessage::NameChanged(s))
        })
        .placeholder("Name of keychain")
        .view();

        let password = TextInput::new("Password", &self.password, |s| {
            Message::Restore(RestoreMessage::PasswordChanged(s))
        })
        .placeholder("Password")
        .password()
        .view();

        let confirm_password = TextInput::new("Confirm password", &self.confirm_password, |s| {
            Message::Restore(RestoreMessage::ConfirmPasswordChanged(s))
        })
        .placeholder("Confirm password")
        .password()
        .view();

        let mnemonic = TextInput::new("Mnemonic (BIP39)", &self.mnemonic, |s| {
            Message::Restore(RestoreMessage::MnemonicChanged(s))
        })
        .placeholder("Mnemonic")
        .view();

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
            .push(if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            })
            .push(restore_keychain_btn)
            .push(Rule::horizontal(1))
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
