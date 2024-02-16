// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeSet;

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;
use smartvaults_sdk::nostr::{Profile, PublicKey};
use smartvaults_sdk::types::backup::PolicyBackup;
use smartvaults_sdk::util;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, ButtonStyle, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum RestoreVaultMessage {
    Load(BTreeSet<Profile>),
    NameChanged(String),
    DescriptionChanged(String),
    SelectPolicyBackup,
    LoadPolicyBackup(PolicyBackup),
    ErrorChanged(Option<String>),
    SavePolicy,
    Clear,
}

#[derive(Debug, Default)]
pub struct RestoreVaultState {
    name: String,
    description: String,
    descriptor: String,
    public_keys: Vec<PublicKey>,
    known_public_keys: BTreeSet<Profile>,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl RestoreVaultState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.descriptor = String::new();
        self.public_keys = Vec::new();
        self.error = None;
    }
}

impl State for RestoreVaultState {
    fn title(&self) -> String {
        String::from("Restore vault")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move { client.get_known_profiles().await.unwrap() },
            |known_public_keys| RestoreVaultMessage::Load(known_public_keys).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::RestorePolicy(msg) = message {
            match msg {
                RestoreVaultMessage::Load(known_public_keys) => {
                    self.known_public_keys = known_public_keys;
                    self.loading = false;
                    self.loaded = true;
                }
                RestoreVaultMessage::NameChanged(name) => self.name = name,
                RestoreVaultMessage::DescriptionChanged(desc) => self.description = desc,
                RestoreVaultMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                RestoreVaultMessage::SelectPolicyBackup => {
                    let path = FileDialog::new()
                        .set_title("Select vault backup")
                        .pick_file();

                    if let Some(path) = path {
                        return Command::perform(async move { PolicyBackup::open(path) }, |res| {
                            match res {
                                Ok(backup) => RestoreVaultMessage::LoadPolicyBackup(backup).into(),
                                Err(e) => {
                                    RestoreVaultMessage::ErrorChanged(Some(e.to_string())).into()
                                }
                            }
                        });
                    }
                }
                RestoreVaultMessage::LoadPolicyBackup(backup) => {
                    if let Some(name) = backup.name() {
                        self.name = name;
                    }
                    if let Some(description) = backup.description() {
                        self.description = description;
                    }
                    self.descriptor = backup.descriptor().to_string();
                    self.public_keys = backup.public_keys();
                }
                RestoreVaultMessage::SavePolicy => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let name = self.name.clone();
                    let description = self.description.clone();
                    let descriptor = self.descriptor.clone();
                    let public_keys = self.public_keys.clone();
                    return Command::perform(
                        async move {
                            client
                                .save_policy(name, description, descriptor, public_keys)
                                .await
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Vaults),
                            Err(e) => RestoreVaultMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                RestoreVaultMessage::Clear => {
                    self.clear();
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let name = TextInput::with_label("Name", &self.name)
            .on_input(|s| RestoreVaultMessage::NameChanged(s).into())
            .placeholder("Vault name")
            .view();

        let description = TextInput::with_label("Description", &self.description)
            .on_input(|s| RestoreVaultMessage::DescriptionChanged(s).into())
            .placeholder("Vault description")
            .view();

        let descriptor = TextInput::with_label("Descriptor", &self.descriptor)
            .placeholder("Vault descriptor")
            .view();

        let mut public_keys = Column::new()
            .push(Text::new("Public Keys").view())
            .spacing(5)
            .width(Length::Fill);

        if self.public_keys.is_empty() {
            public_keys = public_keys.push(Text::new("No public keys").small().view())
        } else {
            for public_key in self.public_keys.iter() {
                public_keys = public_keys.push(
                    Text::new(format!(
                        "- {}",
                        self.known_public_keys
                            .iter()
                            .find(|u| u.public_key() == *public_key)
                            .map(|u| u.name())
                            .unwrap_or_else(|| util::cut_public_key(*public_key)),
                    ))
                    .small()
                    .view(),
                )
            }
        }

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let mut content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("Restore vault").big().bold().view())
                    .push(
                        Text::new("Restore vault from a backup")
                            .extra_light()
                            .view(),
                    )
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(name)
            .push(description)
            .push(descriptor)
            .push(public_keys)
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)));

        if self.descriptor.is_empty() {
            content = content.push(
                Button::new()
                    .style(ButtonStyle::Bordered)
                    .text("Select vault backup")
                    .on_press(RestoreVaultMessage::SelectPolicyBackup.into())
                    .width(Length::Fill)
                    .view(),
            );
        } else {
            content = content
                .push(
                    Button::new()
                        .text("Save vault")
                        .width(Length::Fill)
                        .on_press(RestoreVaultMessage::SavePolicy.into())
                        .loading(self.loading)
                        .view(),
                )
                .push(
                    Button::new()
                        .style(ButtonStyle::BorderedDanger)
                        .text("Clear")
                        .width(Length::Fill)
                        .on_press(RestoreVaultMessage::Clear.into())
                        .loading(self.loading)
                        .view(),
                );
        }

        content = content
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
    }
}

impl From<RestoreVaultState> for Box<dyn State> {
    fn from(s: RestoreVaultState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<RestoreVaultMessage> for Message {
    fn from(msg: RestoreVaultMessage) -> Self {
        Self::RestorePolicy(msg)
    }
}
