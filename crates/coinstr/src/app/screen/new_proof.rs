// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;

use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::db::model::GetPolicy;
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util;
use iced::widget::{Column, Container, PickList, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone, Eq)]
pub struct PolicyPicLisk {
    pub policy_id: EventId,
    pub name: String,
}

impl PartialEq for PolicyPicLisk {
    fn eq(&self, other: &Self) -> bool {
        self.policy_id == other.policy_id
    }
}

impl fmt::Display for PolicyPicLisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - #{}", self.name, util::cut_event_id(self.policy_id))
    }
}

#[derive(Debug, Clone)]
pub enum NewProofMessage {
    LoadPolicies(Vec<PolicyPicLisk>),
    PolicySelectd(PolicyPicLisk),
    MessageChanged(String),
    ErrorChanged(Option<String>),
    SendProposal,
}

#[derive(Debug)]
pub struct NewProofState {
    policy: Option<PolicyPicLisk>,
    policies: Vec<PolicyPicLisk>,
    message: String,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl NewProofState {
    pub fn new(policy: Option<(EventId, Policy)>) -> Self {
        Self {
            policy: policy.map(|(policy_id, policy)| PolicyPicLisk {
                policy_id,
                name: policy.name,
            }),
            policies: Vec::new(),
            message: String::new(),
            loading: false,
            loaded: false,
            error: None,
        }
    }
}

impl State for NewProofState {
    fn title(&self) -> String {
        String::from("New Proof of Reserve")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                client
                    .get_policies()
                    .unwrap()
                    .into_iter()
                    .map(
                        |GetPolicy {
                             policy_id, policy, ..
                         }| PolicyPicLisk {
                            policy_id,
                            name: policy.name,
                        },
                    )
                    .collect()
            },
            |p| NewProofMessage::LoadPolicies(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::NewProof(msg) = message {
            match msg {
                NewProofMessage::LoadPolicies(policies) => {
                    self.policies = policies;
                    self.loading = false;
                    self.loaded = true;
                }
                NewProofMessage::PolicySelectd(policy) => {
                    self.policy = Some(policy);
                }
                NewProofMessage::MessageChanged(value) => self.message = value,
                NewProofMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                NewProofMessage::SendProposal => match &self.policy {
                    Some(policy) => {
                        let client = ctx.client.clone();
                        let policy_id = policy.policy_id;
                        let message = self.message.clone();
                        if !self.message.is_empty() {
                            self.loading = true;
                            return Command::perform(
                                async move { client.new_proof_proposal(policy_id, message).await },
                                |res| match res {
                                    Ok((event_id, ..)) => Message::View(Stage::Proposal(event_id)),
                                    Err(e) => {
                                        NewProofMessage::ErrorChanged(Some(e.to_string())).into()
                                    }
                                },
                            );
                        } else {
                            self.error = Some(String::from("Message can't be empty"));
                        }
                    }
                    None => self.error = Some(String::from("You must select a policy")),
                },
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = if self.loaded {
            let policy_pick_list = Column::new()
                .push(Text::new("Policy").view())
                .push(
                    PickList::new(self.policies.clone(), self.policy.clone(), |policy| {
                        NewProofMessage::PolicySelectd(policy).into()
                    })
                    .width(Length::Fill)
                    .text_size(20)
                    .padding(10)
                    .placeholder(if self.policies.is_empty() {
                        "No policy availabe"
                    } else {
                        "Select a policy"
                    }),
                )
                .spacing(5);

            let message = TextInput::new("Message", &self.message)
                .on_input(|s| NewProofMessage::MessageChanged(s).into())
                .placeholder("Message")
                .view();

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            Column::new()
                .push(
                    Column::new()
                        .push(Text::new("Proof of Reserve").size(24).bold().view())
                        .push(
                            Text::new("Create a new Proof of Reserve")
                                .extra_light()
                                .view(),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(policy_pick_list)
                .push(message)
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(error)
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(
                    Button::new()
                        .text("Send")
                        .width(Length::Fill)
                        .on_press(NewProofMessage::SendProposal.into())
                        .loading(self.loading)
                        .view(),
                )
        } else {
            Column::new().push(Text::new("Loading...").view())
        };

        let content = Container::new(
            content
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400),
        )
        .width(Length::Fill)
        .center_x();

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<NewProofState> for Box<dyn State> {
    fn from(s: NewProofState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<NewProofMessage> for Message {
    fn from(msg: NewProofMessage) -> Self {
        Self::NewProof(msg)
    }
}
