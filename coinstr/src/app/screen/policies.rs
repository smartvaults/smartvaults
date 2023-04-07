// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::fs::File;
use std::io::Write;

use coinstr_core::bdk::miniscript::Descriptor;
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::policy::Policy;
use iced::widget::{Column, Row, Rule};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text};
use crate::theme::icon::{EXPORT, FULLSCREEN, RELOAD};
use crate::APP_NAME;

#[derive(Debug, Clone)]
pub enum PoliciesMessage {
    LoadPolicies(Vec<(EventId, Policy)>),
    ExportDescriptor(Descriptor<String>),
    Reload,
}

#[derive(Debug, Default)]
pub struct PoliciesState {
    loading: bool,
    loaded: bool,
    policies: Vec<(EventId, Policy)>,
}

impl PoliciesState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for PoliciesState {
    fn title(&self) -> String {
        format!("{APP_NAME} - policies")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move { client.get_policies(None).await.unwrap() },
            |p| PoliciesMessage::LoadPolicies(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            self.load(ctx);
        }

        if let Message::PoliciesMessage(msg) = message {
            match msg {
                PoliciesMessage::LoadPolicies(policies) => {
                    self.policies = policies;
                    self.loading = false;
                    self.loaded = true;
                    Command::none()
                }
                PoliciesMessage::ExportDescriptor(desc) => {
                    let path = FileDialog::new()
                        .set_title("Export descriptor backup")
                        .save_file();

                    if let Some(path) = path {
                        match File::create(path) {
                            Ok(mut file) => match file.write_all(desc.to_string().as_bytes()) {
                                Ok(_) => println!("Saved!"),
                                Err(e) => println!("Impossible to save file: {e}"),
                            },
                            Err(e) => println!("Impossible to create file: {e}"),
                        }
                    }
                    Command::none()
                }
                PoliciesMessage::Reload => self.load(ctx),
            }
        } else {
            Command::none()
        }
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            if self.policies.is_empty() {
                content = content.push(Text::new("No policies").view());
                // TODO: add button to create a policy
            } else {
                center_y = false;

                let mut reload_btn = button::border_only_icon(RELOAD).width(Length::Units(40));

                if !self.loading {
                    reload_btn = reload_btn.on_press(PoliciesMessage::Reload.into());
                }

                content = content
                    .push(
                        Row::new()
                            .push(Text::new("ID").bold().width(Length::Fill).view())
                            .push(Text::new("Name").bold().width(Length::Fill).view())
                            .push(Text::new("Description").bold().width(Length::Fill).view())
                            .push(Text::new("").width(Length::Units(40)).view())
                            .push(reload_btn)
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(Rule::horizontal(1));

                for (policy_id, policy) in self.policies.iter() {
                    let row = Row::new()
                        .push(
                            Text::new(cut_policy_id(*policy_id))
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(Text::new(&policy.name).width(Length::Fill).view())
                        .push(Text::new(&policy.description).width(Length::Fill).view())
                        .push(
                            button::border_only_icon(EXPORT)
                                .on_press(
                                    PoliciesMessage::ExportDescriptor(policy.descriptor.clone())
                                        .into(),
                                )
                                .width(Length::Units(40)),
                        )
                        .push(
                            button::primary_only_icon(FULLSCREEN)
                                .on_press(Message::View(Stage::Policy(*policy_id)))
                                .width(Length::Units(40)),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill);
                    content = content.push(row).push(Rule::horizontal(1));
                }
            }
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, center_y)
    }
}

fn cut_policy_id(policy_id: EventId) -> String {
    policy_id.to_string()[..8].to_string()
}

impl From<PoliciesState> for Box<dyn State> {
    fn from(s: PoliciesState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<PoliciesMessage> for Message {
    fn from(msg: PoliciesMessage) -> Self {
        Self::PoliciesMessage(msg)
    }
}
