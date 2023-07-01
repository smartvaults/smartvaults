// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::nostr::Metadata;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text, TextInput};
use crate::constants::APP_NAME;
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum EditProfileMessage {
    LoadMetadata(Metadata),
    NameChanged(String),
    DisplayNameChanged(String),
    NIP05Changed(String),
    ErrorChanged(Option<String>),
    Save,
}

#[derive(Debug, Default)]
pub struct EditProfileState {
    loading: bool,
    loaded: bool,
    current_metadata: Metadata,
    name: String,
    display_name: String,
    nip05: String,
    error: Option<String>,
}

impl EditProfileState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for EditProfileState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Edit profile")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loaded = true;
        let client = ctx.client.clone();
        Command::perform(async move { client.get_profile().unwrap() }, |metadata| {
            EditProfileMessage::LoadMetadata(metadata).into()
        })
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::EditProfile(msg) = message {
            match msg {
                EditProfileMessage::LoadMetadata(metadata) => {
                    self.current_metadata = metadata.clone();
                    if let Some(name) = metadata.name {
                        self.name = name;
                    }
                    if let Some(display_name) = metadata.display_name {
                        self.display_name = display_name;
                    }
                    if let Some(nip05) = metadata.nip05 {
                        self.nip05 = nip05;
                    }
                    self.loading = false;
                    self.loaded = true;
                }
                EditProfileMessage::NameChanged(name) => self.name = name,
                EditProfileMessage::DisplayNameChanged(display_name) => {
                    self.display_name = display_name
                }
                EditProfileMessage::NIP05Changed(nip05) => self.nip05 = nip05,
                EditProfileMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                EditProfileMessage::Save => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let mut metadata = Metadata::new();

                    if !self.name.is_empty() {
                        metadata = metadata.name(&self.name);
                    }

                    if !self.display_name.is_empty() {
                        metadata = metadata.display_name(&self.display_name);
                    }

                    if !self.nip05.is_empty() {
                        metadata = metadata.nip05(&self.nip05);
                    }

                    if metadata != self.current_metadata {
                        return Command::perform(
                            async move { client.set_metadata(metadata).await },
                            |res| match res {
                                Ok(_) => Message::View(Stage::Profile),
                                Err(e) => {
                                    EditProfileMessage::ErrorChanged(Some(e.to_string())).into()
                                }
                            },
                        );
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = if self.loaded {
            let name = TextInput::new("Name", &self.name)
                .on_input(|s| EditProfileMessage::NameChanged(s).into())
                .placeholder("Name")
                .view();

            let display_name = TextInput::new("Display name", &self.display_name)
                .on_input(|s| EditProfileMessage::DisplayNameChanged(s).into())
                .placeholder("Display name")
                .view();

            let nip05 = TextInput::new("NIP-05", &self.nip05)
                .on_input(|s| EditProfileMessage::NIP05Changed(s).into())
                .placeholder("NIP-05")
                .view();

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            let save_contact_btn = button::primary("Save")
                .on_press(EditProfileMessage::Save.into())
                .width(Length::Fill);

            Column::new()
                .push(Text::new("Edit profile").size(24).bold().view())
                .push(name)
                .push(display_name)
                .push(nip05)
                .push(error)
                .push(Space::with_height(Length::Fixed(15.0)))
                .push(save_contact_btn)
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400)
        } else {
            Column::new().push(Text::new("Loading...").view())
        };

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<EditProfileState> for Box<dyn State> {
    fn from(s: EditProfileState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<EditProfileMessage> for Message {
    fn from(msg: EditProfileMessage) -> Self {
        Self::EditProfile(msg)
    }
}