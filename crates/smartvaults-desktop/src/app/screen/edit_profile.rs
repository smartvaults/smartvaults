// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::nostr::Metadata;
use smartvaults_sdk::protocol::v1::KeyAgentMetadata;

use crate::app::component::Dashboard;
use crate::app::context::Mode;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum EditProfileMessage {
    LoadMetadata(Box<Metadata>),
    NameChanged(String),
    DisplayNameChanged(String),
    NIP05Changed(String),
    JurisdictionChanged(String),
    TwitterChanged(String),
    FacebookChanged(String),
    LinkedinChanged(String),
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
    jurisdiction: String,
    twitter: String,
    facebook: String,
    linkedin: String,
    error: Option<String>,
}

impl EditProfileState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for EditProfileState {
    fn title(&self) -> String {
        String::from("Edit profile")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move { client.get_profile().await.unwrap().metadata() },
            |metadata| EditProfileMessage::LoadMetadata(Box::new(metadata)).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::EditProfile(msg) = message {
            match msg {
                EditProfileMessage::LoadMetadata(metadata) => {
                    self.current_metadata = *metadata.clone();
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
                EditProfileMessage::JurisdictionChanged(jurisdiction) => {
                    self.jurisdiction = jurisdiction
                }
                EditProfileMessage::TwitterChanged(twitter) => self.twitter = twitter,
                EditProfileMessage::FacebookChanged(facebook) => self.facebook = facebook,
                EditProfileMessage::LinkedinChanged(linkedin) => self.linkedin = linkedin,
                EditProfileMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                EditProfileMessage::Save => {
                    self.loading = true;
                    let client = ctx.client.clone();

                    let mut metadata = match ctx.mode {
                        Mode::User => Metadata::new(),
                        Mode::KeyAgent => {
                            let mut key_agent_metadata = KeyAgentMetadata::default();

                            if !self.jurisdiction.is_empty() {
                                key_agent_metadata.jurisdiction = Some(self.jurisdiction.clone());
                            }

                            key_agent_metadata.into_metadata()
                        }
                    };

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
                            async move { client.set_metadata(&metadata).await },
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
        let mut content = Column::new();

        if self.loaded {
            let name = TextInput::with_label("Name", &self.name)
                .on_input(|s| EditProfileMessage::NameChanged(s).into())
                .placeholder("Name")
                .view();

            let display_name = TextInput::with_label("Display name", &self.display_name)
                .on_input(|s| EditProfileMessage::DisplayNameChanged(s).into())
                .placeholder("Display name")
                .view();

            let nip05 = TextInput::with_label("NIP-05", &self.nip05)
                .on_input(|s| EditProfileMessage::NIP05Changed(s).into())
                .placeholder("NIP-05")
                .view();

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            content = content
                .push(Text::new("Edit profile").big().bold().view())
                .push(name)
                .push(display_name)
                .push(nip05);

            if let Mode::KeyAgent = ctx.mode {
                let jurisdiction = TextInput::with_label("Jurisdiction", &self.jurisdiction)
                    .on_input(|s| EditProfileMessage::JurisdictionChanged(s).into())
                    .placeholder("Jurisdiction")
                    .view();

                let twitter = TextInput::with_label("X (Twitter)", &self.twitter)
                    .on_input(|s| EditProfileMessage::TwitterChanged(s).into())
                    .placeholder("X (Twitter)")
                    .view();

                let facebook = TextInput::with_label("Facebook", &self.facebook)
                    .on_input(|s| EditProfileMessage::FacebookChanged(s).into())
                    .placeholder("Facebook")
                    .view();

                let linkedin = TextInput::with_label("Linkedin", &self.linkedin)
                    .on_input(|s| EditProfileMessage::LinkedinChanged(s).into())
                    .placeholder("Linkedin")
                    .view();

                content = content
                    .push(jurisdiction)
                    .push(twitter)
                    .push(facebook)
                    .push(linkedin);
            }

            content = content
                .push(error)
                .push(Space::with_height(Length::Fixed(15.0)))
                .push(
                    Button::new()
                        .text("Save")
                        .on_press(EditProfileMessage::Save.into())
                        .loading(self.loading)
                        .width(Length::Fill)
                        .view(),
                )
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400);
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
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
