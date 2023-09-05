// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;

use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::db::model::{GetAddress, GetPolicy};
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::protocol::v1::Label;
use coinstr_sdk::util;
use iced::widget::qr_code::{self, QRCode};
use iced::widget::{Column, PickList, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{Button, ButtonStyle, Text, TextInput};
use crate::theme::icon::CLIPBOARD;

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
pub enum ReceiveMessage {
    LoadPolicies(Vec<PolicyPicLisk>),
    LoadAddress(EventId),
    PolicySelectd(PolicyPicLisk),
    AddressChanged(GetAddress),
    LabelChanged(String),
    SaveLabel(Label),
    ErrorChanged(Option<String>),
    Reload,
}

#[derive(Debug)]
pub struct ReceiveState {
    policy: Option<PolicyPicLisk>,
    policies: Vec<PolicyPicLisk>,
    qr_code: Option<qr_code::State>,
    address: Option<GetAddress>,
    label: String,
    loading: bool,
    loaded: bool,
    allow_reload: bool,
    error: Option<String>,
}

impl ReceiveState {
    pub fn new(policy: Option<(EventId, Policy)>) -> Self {
        Self {
            policy: policy.map(|(policy_id, policy)| PolicyPicLisk {
                policy_id,
                name: policy.name,
            }),
            policies: Vec::new(),
            qr_code: None,
            address: None,
            label: String::new(),
            loading: false,
            loaded: false,
            allow_reload: false,
            error: None,
        }
    }
}

impl State for ReceiveState {
    fn title(&self) -> String {
        String::from("Receive")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loaded && !self.allow_reload {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                client
                    .get_policies()
                    .await
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
            |p| ReceiveMessage::LoadPolicies(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Receive(msg) = message {
            match msg {
                ReceiveMessage::LoadPolicies(policies) => {
                    self.policies = policies;
                    self.loading = false;
                    self.loaded = true;
                    self.allow_reload = false;
                    if let Some(policy) = self.policy.as_ref() {
                        let policy_id = policy.policy_id;
                        return Command::perform(async {}, move |_| {
                            ReceiveMessage::LoadAddress(policy_id).into()
                        });
                    }
                }
                ReceiveMessage::LoadAddress(policy_id) => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.get_last_unused_address(policy_id).await },
                        |res| match res {
                            Ok(address) => ReceiveMessage::AddressChanged(address).into(),
                            Err(e) => ReceiveMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ReceiveMessage::PolicySelectd(policy) => {
                    let policy_id = policy.policy_id;
                    self.policy = Some(policy);
                    return Command::perform(async {}, move |_| {
                        ReceiveMessage::LoadAddress(policy_id).into()
                    });
                }
                ReceiveMessage::AddressChanged(value) => {
                    self.label = value.label.clone().unwrap_or_default();
                    self.address = Some(value);
                    if let Some(address) = self.address.clone() {
                        let address = address.address.clone();
                        self.qr_code =
                            qr_code::State::new(address.assume_checked().to_qr_uri()).ok();
                    }
                }
                ReceiveMessage::LabelChanged(label) => self.label = label,
                ReceiveMessage::SaveLabel(label) => {
                    let client = ctx.client.clone();
                    if let Some(policy) = self.policy.as_ref() {
                        self.loading = false;
                        let policy_id = policy.policy_id;
                        return Command::perform(
                            async move { client.save_label(policy_id, label).await },
                            |res| match res {
                                Ok(_) => ReceiveMessage::Reload.into(),
                                Err(e) => ReceiveMessage::ErrorChanged(Some(e.to_string())).into(),
                            },
                        );
                    } else {
                        self.error = Some(String::from("Policy not selected"));
                    }
                }
                ReceiveMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                ReceiveMessage::Reload => {
                    self.allow_reload = true;
                    return self.load(ctx);
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new();

        if self.loaded {
            content = content
                .push(
                    Column::new()
                        .push(Text::new("Receive").big().bold().view())
                        .push(
                            Text::new("Send sats to the address below to fund your wallet.")
                                .extra_light()
                                .view(),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(
                    Column::new()
                        .push(Text::new("Policy").view())
                        .push(
                            PickList::new(self.policies.clone(), self.policy.clone(), |policy| {
                                ReceiveMessage::PolicySelectd(policy).into()
                            })
                            .width(Length::Fill)
                            .padding(10)
                            .placeholder(
                                if self.policies.is_empty() {
                                    "No policy availabe"
                                } else {
                                    "Select a policy"
                                },
                            ),
                        )
                        .spacing(5),
                );

            if let Some(address) = self.address.clone() {
                content = content
                    .push(
                        TextInput::new("Label", &self.label)
                            .on_input(|l| ReceiveMessage::LabelChanged(l).into())
                            .placeholder("Label")
                            .button(
                                Button::new()
                                    .style(ButtonStyle::Bordered)
                                    .text("Save")
                                    .width(Length::Fixed(80.0))
                                    .loading(
                                        self.loading
                                            || self.label.is_empty()
                                            || self
                                                .address
                                                .as_ref()
                                                .map(|a| a.label.clone().unwrap_or_default())
                                                .eq(&Some(self.label.clone())),
                                    )
                                    .on_press(
                                        ReceiveMessage::SaveLabel(Label::address(
                                            address.address.clone(),
                                            self.label.clone(),
                                        ))
                                        .into(),
                                    )
                                    .view(),
                            )
                            .view(),
                    )
                    .push(Space::with_height(Length::Fixed(20.0)));

                if let Some(qr_code) = self.qr_code.as_ref() {
                    content = content
                        .push(QRCode::new(qr_code).cell_size(5))
                        .push(Space::with_height(Length::Fixed(10.0)));
                };

                let address = address.address.clone().assume_checked();

                let mut address_splitted = String::new();
                for (index, char) in address.to_string().char_indices() {
                    if index % 4 == 0 {
                        address_splitted.push(' ');
                    }
                    address_splitted.push(char);
                }

                content = content
                    .push(Text::new(address_splitted).extra_light().view())
                    .push(Space::with_height(Length::Fixed(10.0)))
                    .push(
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .icon(CLIPBOARD)
                            .text("Copy")
                            .width(Length::Fill)
                            .on_press(Message::Clipboard(address.to_string()))
                            .view(),
                    );
            }

            content = content
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400)
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
    }
}

impl From<ReceiveState> for Box<dyn State> {
    fn from(s: ReceiveState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ReceiveMessage> for Message {
    fn from(msg: ReceiveMessage) -> Self {
        Self::Receive(msg)
    }
}
