// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};

use coinstr_sdk::core::secp256k1::XOnlyPublicKey;
use coinstr_sdk::nostr::Metadata;
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text, TextInput};
use crate::theme::color::DARK_RED;
use crate::theme::icon::PLUS;

#[derive(Debug, Clone)]
pub enum AddPolicyMessage {
    NameChanged(String),
    DescriptionChanged(String),
    DescriptorChanged(String),
    LoadContacts(BTreeMap<XOnlyPublicKey, Metadata>),
    AddPublicKey(XOnlyPublicKey),
    RemovePublicKey(XOnlyPublicKey),
    SelectPublicKeys(bool),
    ErrorChanged(Option<String>),
    SavePolicy,
}

#[derive(Debug, Default)]
pub struct AddPolicyState {
    name: String,
    description: String,
    descriptor: String,
    contacts: BTreeMap<XOnlyPublicKey, Metadata>,
    public_keys: HashSet<XOnlyPublicKey>,
    loading: bool,
    loaded: bool,
    selecting: bool,
    error: Option<String>,
}

impl AddPolicyState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddPolicyState {
    fn title(&self) -> String {
        String::from("Add policy")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let mut contacts = client.get_contacts().await.unwrap();
                contacts.insert(
                    client.keys().public_key(),
                    client.get_profile().await.unwrap(),
                );
                contacts
            },
            |p| AddPolicyMessage::LoadContacts(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::AddPolicy(msg) = message {
            match msg {
                AddPolicyMessage::NameChanged(name) => self.name = name,
                AddPolicyMessage::DescriptionChanged(desc) => self.description = desc,
                AddPolicyMessage::DescriptorChanged(desc) => self.descriptor = desc,
                AddPolicyMessage::LoadContacts(contacts) => {
                    self.contacts = contacts;
                    self.loading = false;
                    self.loaded = true;
                }
                AddPolicyMessage::SelectPublicKeys(value) => self.selecting = value,
                AddPolicyMessage::AddPublicKey(public_key) => {
                    self.public_keys.insert(public_key);
                }
                AddPolicyMessage::RemovePublicKey(public_key) => {
                    self.public_keys.remove(&public_key);
                }
                AddPolicyMessage::ErrorChanged(error) => self.error = error,
                AddPolicyMessage::SavePolicy => {
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
                            Ok(_) => Message::View(Stage::Policies),
                            Err(e) => AddPolicyMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut center_y = true;

        let name = TextInput::new("Name", &self.name)
            .on_input(|s| AddPolicyMessage::NameChanged(s).into())
            .placeholder("Policy name")
            .view();

        let description = TextInput::new("Description", &self.description)
            .on_input(|s| AddPolicyMessage::DescriptionChanged(s).into())
            .placeholder("Policy description")
            .view();

        let descriptor = TextInput::new("Descriptor/Policy", &self.descriptor)
            .on_input(|s| AddPolicyMessage::DescriptorChanged(s).into())
            .placeholder("Policy descriptor")
            .view();

        let mut public_keys = Column::new()
            .push(Text::new("Public Keys (optional)").view())
            .spacing(5);

        if !self.public_keys.is_empty() {
            for public_key in self.public_keys.iter() {
                public_keys = public_keys.push(
                    Text::new(format!(
                        "- {}{}",
                        // TODO: ctx.client.db.get_public_key_name(*public_key),
                        "TODO",
                        if ctx.client.keys().public_key() == *public_key {
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
                    .on_press(AddPolicyMessage::SelectPublicKeys(true).into())
                    .view(),
            );

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let save_policy_btn = Button::new()
            .text("Save policy")
            .on_press(AddPolicyMessage::SavePolicy.into())
            .width(Length::Fill);

        let restore_policy_btn = Button::new()
            .style(ButtonStyle::Bordered)
            .text("Restore policy backup")
            .on_press(Message::View(Stage::RestorePolicy))
            .width(Length::Fill);

        let policy_builder_btn = Button::new()
            .style(ButtonStyle::Bordered)
            .text("Policy builder")
            .on_press(Message::View(Stage::PolicyBuilder))
            .width(Length::Fill);

        let content = if self.selecting {
            center_y = false;
            view_select_public_keys(self, ctx)
        } else {
            Column::new()
                .push(
                    Column::new()
                        .push(Text::new("Create policy").big().bold().view())
                        .push(Text::new("Create a new policy").extra_light().view())
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

fn view_select_public_keys<'a>(state: &AddPolicyState, ctx: &Context) -> Column<'a, Message> {
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

        for (public_key, metadata) in state.contacts.iter() {
            let select_btn = if state.public_keys.contains(public_key) {
                Button::new()
                    .text("Selected")
                    .on_press(AddPolicyMessage::RemovePublicKey(*public_key).into())
            } else {
                Button::new()
                    .style(ButtonStyle::Bordered)
                    .text("Select")
                    .on_press(AddPolicyMessage::AddPublicKey(*public_key).into())
            };

            let row = Row::new()
                .push(
                    Text::new(format!(
                        "{}{}",
                        util::cut_public_key(*public_key),
                        if ctx.client.keys().public_key() == *public_key {
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
                        .on_press(AddPolicyMessage::SelectPublicKeys(false).into())
                        .view(),
                )
                .width(Length::Fill)
                .align_items(Alignment::End),
        );
    }

    content
}

impl From<AddPolicyState> for Box<dyn State> {
    fn from(s: AddPolicyState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddPolicyMessage> for Message {
    fn from(msg: AddPolicyMessage) -> Self {
        Self::AddPolicy(msg)
    }
}
