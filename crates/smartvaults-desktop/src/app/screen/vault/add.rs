// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::{HashSet, VecDeque};

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::secp256k1::XOnlyPublicKey;
use smartvaults_sdk::types::User;
use smartvaults_sdk::util;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text, TextInput};
use crate::theme::color::DARK_RED;
use crate::theme::icon::PLUS;

#[derive(Debug, Clone)]
pub enum AddVaultMessage {
    NameChanged(String),
    DescriptionChanged(String),
    DescriptorChanged(String),
    Load(Box<User>, VecDeque<User>),
    AddPublicKey(XOnlyPublicKey),
    RemovePublicKey(XOnlyPublicKey),
    SelectPublicKeys(bool),
    ErrorChanged(Option<String>),
    SavePolicy,
}

#[derive(Debug, Default)]
pub struct AddVaultState {
    name: String,
    description: String,
    descriptor: String,
    profile: Option<User>,
    contacts: VecDeque<User>,
    public_keys: HashSet<XOnlyPublicKey>,
    loading: bool,
    loaded: bool,
    selecting: bool,
    error: Option<String>,
}

impl AddVaultState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddVaultState {
    fn title(&self) -> String {
        String::from("Add vault")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let profile = client.get_profile().await.unwrap();
                let mut contacts: VecDeque<User> =
                    client.get_contacts().await.unwrap().into_iter().collect();
                contacts.push_front(profile.clone());
                (profile, contacts)
            },
            |(profile, contacts)| AddVaultMessage::Load(Box::new(profile), contacts).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::AddPolicy(msg) = message {
            match msg {
                AddVaultMessage::NameChanged(name) => self.name = name,
                AddVaultMessage::DescriptionChanged(desc) => self.description = desc,
                AddVaultMessage::DescriptorChanged(desc) => self.descriptor = desc,
                AddVaultMessage::Load(profile, contacts) => {
                    self.profile = Some(*profile);
                    self.contacts = contacts;
                    self.loading = false;
                    self.loaded = true;
                }
                AddVaultMessage::SelectPublicKeys(value) => self.selecting = value,
                AddVaultMessage::AddPublicKey(public_key) => {
                    self.public_keys.insert(public_key);
                }
                AddVaultMessage::RemovePublicKey(public_key) => {
                    self.public_keys.remove(&public_key);
                }
                AddVaultMessage::ErrorChanged(error) => self.error = error,
                AddVaultMessage::SavePolicy => {
                    let client = ctx.client.clone();
                    let name = self.name.clone();
                    let description = self.description.clone();
                    let descriptor = self.descriptor.clone();
                    let public_keys: Vec<XOnlyPublicKey> =
                        self.public_keys.iter().copied().collect();
                    return Command::perform(
                        async move {
                            client
                                .save_policy(name, description, descriptor, public_keys)
                                .await
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Vaults),
                            Err(e) => AddVaultMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut center_y = true;

        let name = TextInput::with_label("Name", &self.name)
            .on_input(|s| AddVaultMessage::NameChanged(s).into())
            .placeholder("Vault name")
            .view();

        let description = TextInput::with_label("Description", &self.description)
            .on_input(|s| AddVaultMessage::DescriptionChanged(s).into())
            .placeholder("Vault description")
            .view();

        let descriptor = TextInput::with_label("Descriptor/Policy", &self.descriptor)
            .on_input(|s| AddVaultMessage::DescriptorChanged(s).into())
            .placeholder("Vault descriptor")
            .view();

        let mut public_keys = Column::new()
            .push(Text::new("Public Keys (optional)").view())
            .spacing(5);

        if !self.public_keys.is_empty() {
            for public_key in self.public_keys.iter() {
                public_keys = public_keys.push(
                    Text::new(format!(
                        "- {}{}",
                        self.contacts
                            .iter()
                            .find(|c| c.public_key() == *public_key)
                            .map(|u| u.name())
                            .unwrap_or_else(|| util::cut_public_key(*public_key)),
                        if self.profile.as_ref().map(|p| p.public_key()) == Some(*public_key) {
                            " (me)"
                        } else {
                            ""
                        }
                    ))
                    .small()
                    .view(),
                )
            }
        }

        public_keys = public_keys
            .push(Space::with_height(Length::Fixed(5.0)))
            .push(
                Button::new()
                    .style(ButtonStyle::Bordered)
                    .text("Select")
                    .width(Length::Fill)
                    .on_press(AddVaultMessage::SelectPublicKeys(true).into())
                    .view(),
            );

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let save_policy_btn = Button::new()
            .text("Save vault")
            .on_press(AddVaultMessage::SavePolicy.into())
            .width(Length::Fill);

        let restore_policy_btn = Button::new()
            .style(ButtonStyle::Bordered)
            .text("Restore vault backup")
            .on_press(Message::View(Stage::RestoreVault))
            .width(Length::Fill);

        let policy_builder_btn = Button::new()
            .style(ButtonStyle::Bordered)
            .text("Vault builder")
            .on_press(Message::View(Stage::VaultBuilder))
            .width(Length::Fill);

        let content = if self.selecting {
            center_y = false;
            view_select_public_keys(self)
        } else {
            Column::new()
                .push(
                    Column::new()
                        .push(Text::new("Create vault").big().bold().view())
                        .push(Text::new("Create a new vault").extra_light().view())
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(name)
                .push(description)
                .push(descriptor)
                .push(public_keys)
                .push(error)
                .push(Space::with_height(Length::Fixed(15.0)))
                .push(save_policy_btn.view())
                .push(restore_policy_btn.view())
                .push(policy_builder_btn.view())
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400)
        };

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
    }
}

fn view_select_public_keys<'a>(state: &AddVaultState) -> Column<'a, Message> {
    let mut content = Column::new().spacing(10).padding(20);

    if state.contacts.is_empty() {
        content = content
            .push(Text::new("No contacts").view())
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(
                Button::new()
                    .icon(PLUS)
                    .text("Add contacts")
                    .on_press(Message::View(Stage::AddContact))
                    .width(Length::Fixed(250.0))
                    .view(),
            )
            .align_items(Alignment::Center);
    } else {
        content = content
            .push(Text::new("Select public keys").big().bold().view())
            .push(Space::with_height(Length::Fixed(30.0)))
            .push(
                Row::new()
                    .push(
                        Text::new("Public Key")
                            .bold()
                            .big()
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(Text::new("Name").bold().big().width(Length::Fill).view())
                    .push(
                        Text::new("Display Name")
                            .bold()
                            .big()
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(Space::with_width(Length::Fixed(180.0)))
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(rule::horizontal_bold());

        for user in state.contacts.iter() {
            let public_key = user.public_key();
            let metadata = user.metadata();

            let select_btn = if state.public_keys.contains(&public_key) {
                Button::new()
                    .text("Selected")
                    .on_press(AddVaultMessage::RemovePublicKey(public_key).into())
            } else {
                Button::new()
                    .style(ButtonStyle::Bordered)
                    .text("Select")
                    .on_press(AddVaultMessage::AddPublicKey(public_key).into())
            };

            let row = Row::new()
                .push(
                    Text::new(format!(
                        "{}{}",
                        util::cut_public_key(public_key),
                        if state.profile.as_ref().map(|p| p.public_key()) == Some(public_key) {
                            " (me)"
                        } else {
                            ""
                        }
                    ))
                    .width(Length::Fill)
                    .view(),
                )
                .push(
                    Text::new(metadata.name.as_deref().unwrap_or_default())
                        .width(Length::Fill)
                        .view(),
                )
                .push(
                    Text::new(metadata.display_name.as_deref().unwrap_or_default())
                        .width(Length::Fill)
                        .view(),
                )
                .push(select_btn.width(Length::Fixed(180.0)).view())
                .spacing(10)
                .align_items(Alignment::Center)
                .width(Length::Fill);
            content = content.push(row).push(rule::horizontal());
        }

        content = content.push(Space::with_height(Length::Fixed(20.0))).push(
            Column::new()
                .push(
                    Button::new()
                        .text("Confirm")
                        .width(Length::Fixed(180.0))
                        .on_press(AddVaultMessage::SelectPublicKeys(false).into())
                        .view(),
                )
                .width(Length::Fill)
                .align_items(Alignment::End),
        );
    }

    content
}

impl From<AddVaultState> for Box<dyn State> {
    fn from(s: AddVaultState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddVaultMessage> for Message {
    fn from(msg: AddVaultMessage) -> Self {
        Self::AddPolicy(msg)
    }
}
