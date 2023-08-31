// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::secp256k1::XOnlyPublicKey;
use coinstr_sdk::types::backup::PolicyBackup;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, ButtonStyle, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum RestorePolicyMessage {
    NameChanged(String),
    DescriptionChanged(String),
    SelectPolicyBackup,
    LoadPolicyBackup(PolicyBackup),
    ErrorChanged(Option<String>),
    SavePolicy,
    Clear,
}

#[derive(Debug, Default)]
pub struct RestorePolicyState {
    name: String,
    description: String,
    descriptor: String,
    public_keys: Vec<XOnlyPublicKey>,
    loading: bool,
    error: Option<String>,
}

impl RestorePolicyState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.descriptor = String::new();
        self.public_keys = Vec::new();
        self.error = None;
    }
}

impl State for RestorePolicyState {
    fn title(&self) -> String {
        String::from("Restore policy")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::RestorePolicy(msg) = message {
            match msg {
                RestorePolicyMessage::NameChanged(name) => self.name = name,
                RestorePolicyMessage::DescriptionChanged(desc) => self.description = desc,
                RestorePolicyMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                RestorePolicyMessage::SelectPolicyBackup => {
                    let path = FileDialog::new()
                        .set_title("Select policy backup")
                        .pick_file();

                    if let Some(path) = path {
                        return Command::perform(async move { PolicyBackup::open(path) }, |res| {
                            match res {
                                Ok(backup) => RestorePolicyMessage::LoadPolicyBackup(backup).into(),
                                Err(e) => {
                                    RestorePolicyMessage::ErrorChanged(Some(e.to_string())).into()
                                }
                            }
                        });
                    }
                }
                RestorePolicyMessage::LoadPolicyBackup(backup) => {
                    if let Some(name) = backup.name() {
                        self.name = name;
                    }
                    if let Some(description) = backup.description() {
                        self.description = description;
                    }
                    self.descriptor = backup.descriptor().to_string();
                    self.public_keys = backup.public_keys();
                }
                RestorePolicyMessage::SavePolicy => {
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
                            Ok(_) => Message::View(Stage::Policies),
                            Err(e) => {
                                RestorePolicyMessage::ErrorChanged(Some(e.to_string())).into()
                            }
                        },
                    );
                }
                RestorePolicyMessage::Clear => {
                    self.clear();
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let name = TextInput::new("Name", &self.name)
            .on_input(|s| RestorePolicyMessage::NameChanged(s).into())
            .placeholder("Policy name")
            .view();

        let description = TextInput::new("Description", &self.description)
            .on_input(|s| RestorePolicyMessage::DescriptionChanged(s).into())
            .placeholder("Policy description")
            .view();

        let descriptor = TextInput::new("Descriptor", &self.descriptor)
            .placeholder("Policy descriptor")
            .view();

        let mut public_keys = Column::new()
            .push(Text::new("Public Keys").view())
            .spacing(5)
            .width(Length::Fill);

        if self.public_keys.is_empty() {
            public_keys = public_keys.push(Text::new("No public keys").small().view())
        } else {
            for _public_key in self.public_keys.iter() {
                public_keys = public_keys.push(
                    Text::new(format!(
                        "- {}",
                        "TODO",
                        //ctx.client.db.get_public_key_name(*public_key)
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
                    .push(Text::new("Restore policy").big().bold().view())
                    .push(
                        Text::new("Restore policy from a backup")
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
                    .text("Select policy backup")
                    .on_press(RestorePolicyMessage::SelectPolicyBackup.into())
                    .width(Length::Fill)
                    .view(),
            );
        } else {
            content = content
                .push(
                    Button::new()
                        .text("Save policy")
                        .width(Length::Fill)
                        .on_press(RestorePolicyMessage::SavePolicy.into())
                        .loading(self.loading)
                        .view(),
                )
                .push(
                    Button::new()
                        .style(ButtonStyle::BorderedDanger)
                        .text("Clear")
                        .width(Length::Fill)
                        .on_press(RestorePolicyMessage::Clear.into())
                        .loading(self.loading)
                        .view(),
                );
        }

        content = content
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<RestorePolicyState> for Box<dyn State> {
    fn from(s: RestorePolicyState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<RestorePolicyMessage> for Message {
    fn from(msg: RestorePolicyMessage) -> Self {
        Self::RestorePolicy(msg)
    }
}
