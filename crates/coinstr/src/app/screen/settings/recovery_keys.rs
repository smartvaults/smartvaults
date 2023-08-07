// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::types::Secrets;
use iced::widget::Column;
use iced::{Alignment, Command, Element};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::TextInput;

#[derive(Debug, Clone)]
pub enum RecoveryKeysMessage {
    Load(Secrets),
    Null,
}

#[derive(Debug, Default)]
pub struct RecoveryKeysState {
    secrets: Option<Secrets>,
    loading: bool,
    loaded: bool,
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

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move { client.keychain().secrets(client.network()) },
            |res| match res {
                Ok(secrets) => RecoveryKeysMessage::Load(secrets).into(),
                Err(e) => {
                    tracing::error!("impossible to load secrets: {e}");
                    Message::View(Stage::Settings)
                }
            },
        )
    }

    fn update(&mut self, _ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::RecoveryKeys(msg) = message {
            match msg {
                RecoveryKeysMessage::Load(secrets) => {
                    self.secrets = Some(secrets);
                    self.loaded = true;
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

        if let Some(secrets) = self.secrets.clone() {
            content = content
                .push(
                    TextInput::new(
                        format!("Entorpy ({} bits)", secrets.entropy.len() / 2 * 8),
                        secrets.entropy,
                    )
                    .on_input(|_| RecoveryKeysMessage::Null.into())
                    .view(),
                )
                .push(
                    TextInput::new("Mnemonic (BIP39)", &secrets.mnemonic.to_string())
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                );

            if let Some(passphrase) = secrets.passphrase {
                content = content.push(
                    TextInput::new("Passphrase", &passphrase)
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                );
            }

            content = content
                .push(
                    TextInput::new("Seed HEX", &secrets.seed_hex)
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                )
                .push(
                    TextInput::new("Root Key (BIP32)", &secrets.root_key.to_string())
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                )
                .push(
                    TextInput::new("Fingerprint (BIP32)", &secrets.fingerprint.to_string())
                        .on_input(|_| RecoveryKeysMessage::Null.into())
                        .view(),
                );
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, false, false)
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
