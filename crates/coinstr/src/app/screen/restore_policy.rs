// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bitcoin::XOnlyPublicKey;
use coinstr_sdk::types::backup::PolicyBackup;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text, TextInput};
use crate::constants::APP_NAME;
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
        format!("{APP_NAME} - Restore policy")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::RestorePolicy(msg) = message {
            match msg {
                RestorePolicyMessage::NameChanged(name) => self.name = name,
                RestorePolicyMessage::DescriptionChanged(desc) => self.description = desc,
                RestorePolicyMessage::ErrorChanged(error) => self.error = error,
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
                    self.descriptor = backup.descriptor().to_string();
                    self.public_keys = backup.public_keys();
                }
                RestorePolicyMessage::SavePolicy => {
                    let client = ctx.client.clone();
                    let name = self.name.clone();
                    let description = self.description.clone();
                    let descriptor = self.descriptor.clone();
                    let public_keys = self.public_keys.clone();
                    return Command::perform(
                        async move {
                            client
                                .save_policy(name, description, descriptor, Some(public_keys))
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

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let mut content = Column::new()
            .push(
                Column::new()
                    .push(Text::new("Restore policy").size(24).bold().view())
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
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)));

        if self.descriptor.is_empty() {
            let select_policy_btn = button::border("Select policy backup")
                .on_press(RestorePolicyMessage::SelectPolicyBackup.into())
                .width(Length::Fill);
            content = content.push(select_policy_btn);
        } else {
            let save_policy_btn = button::primary("Save policy")
                .on_press(RestorePolicyMessage::SavePolicy.into())
                .width(Length::Fill);
            let clear_policy_btn = button::danger_border("Clear")
                .on_press(RestorePolicyMessage::Clear.into())
                .width(Length::Fill);
            content = content.push(save_policy_btn).push(clear_policy_btn);
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
