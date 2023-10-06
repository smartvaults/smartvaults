// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::Column;
use iced::{Command, Element, Length};
use smartvaults_sdk::types::User;
use smartvaults_sdk::util;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, ButtonStyle, Text};
use crate::theme::icon::CLIPBOARD;

#[derive(Debug, Clone)]
pub enum ProfileMessage {
    LoadProfile { user: User },
}

#[derive(Debug, Default)]
pub struct ProfileState {
    loading: bool,
    loaded: bool,
    user: Option<User>,
}

impl ProfileState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ProfileState {
    fn title(&self) -> String {
        String::from("Profile")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loaded = true;
        let client = ctx.client.clone();
        Command::perform(async move { client.get_profile().await.unwrap() }, |user| {
            ProfileMessage::LoadProfile { user }.into()
        })
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Profile(msg) = message {
            match msg {
                ProfileMessage::LoadProfile { user } => {
                    self.user = Some(user);
                    self.loading = false;
                    self.loaded = true;
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        if let Some(user) = &self.user {
            let public_key = user.public_key();
            let metadata = user.metadata();
            content = content
                .push(Text::new(util::cut_public_key(public_key)).view())
                .push(
                    Button::new()
                        .style(ButtonStyle::Bordered)
                        .icon(CLIPBOARD)
                        .on_press(Message::Clipboard(public_key.to_string()))
                        .width(Length::Fixed(40.0))
                        .view(),
                )
                .push(
                    Text::new(format!(
                        "Name: {}",
                        metadata.name.clone().unwrap_or_default()
                    ))
                    .view(),
                )
                .push(
                    Text::new(format!(
                        "Display name: {}",
                        metadata.display_name.clone().unwrap_or_default()
                    ))
                    .view(),
                )
                .push(
                    Text::new(format!(
                        "NIP-05: {}",
                        metadata.nip05.clone().unwrap_or_default()
                    ))
                    .view(),
                )
                .push(
                    Button::new()
                        .style(ButtonStyle::Bordered)
                        .text("Edit profile")
                        .on_press(Message::View(Stage::EditProfile))
                        .view(),
                );
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, false, false)
    }
}

impl From<ProfileState> for Box<dyn State> {
    fn from(s: ProfileState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ProfileMessage> for Message {
    fn from(msg: ProfileMessage) -> Self {
        Self::Profile(msg)
    }
}
