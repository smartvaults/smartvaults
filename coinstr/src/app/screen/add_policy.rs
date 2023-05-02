// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text, TextInput};
use crate::constants::APP_NAME;
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum AddPolicyMessage {
    NameChanged(String),
    DescriptionChanged(String),
    DescriptorChanged(String),
    ErrorChanged(Option<String>),
    SavePolicy,
}

#[derive(Debug, Default)]
pub struct AddPolicyState {
    name: String,
    description: String,
    descriptor: String,
    error: Option<String>,
}

impl AddPolicyState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for AddPolicyState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Add policy")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::AddPolicy(msg) = message {
            match msg {
                AddPolicyMessage::NameChanged(name) => self.name = name,
                AddPolicyMessage::DescriptionChanged(desc) => self.description = desc,
                AddPolicyMessage::DescriptorChanged(desc) => self.descriptor = desc,
                AddPolicyMessage::ErrorChanged(error) => self.error = error,
                AddPolicyMessage::SavePolicy => {
                    let client = ctx.client.clone();
                    let name = self.name.clone();
                    let description = self.description.clone();
                    let descriptor = self.descriptor.clone();
                    return Command::perform(
                        async move { client.save_policy(name, description, descriptor).await },
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

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let save_policy_btn =
            button::primary("Save policy").on_press(AddPolicyMessage::SavePolicy.into());

        let content = Column::new()
            .push(name)
            .push(description)
            .push(descriptor)
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(save_policy_btn)
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
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
