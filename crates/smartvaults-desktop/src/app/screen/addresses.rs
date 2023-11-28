// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashMap;
use std::fmt;

use iced::alignment::Horizontal;
use iced::widget::{Column, PickList, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::bitcoin::ScriptBuf;
use smartvaults_sdk::core::policy::Policy;
use smartvaults_sdk::nostr::EventId;
use smartvaults_sdk::types::{GetAddress, GetPolicy};
use smartvaults_sdk::util;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::icon::CLIPBOARD;

#[derive(Debug, Clone, Eq)]
pub struct PolicyPickList {
    pub policy_id: EventId,
    pub name: String,
}

impl PartialEq for PolicyPickList {
    fn eq(&self, other: &Self) -> bool {
        self.policy_id == other.policy_id
    }
}

impl fmt::Display for PolicyPickList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - #{}", self.name, util::cut_event_id(self.policy_id))
    }
}

#[derive(Debug, Clone)]
pub enum AddressesMessage {
    LoadPolicies(Vec<PolicyPickList>),
    LoadAddresses(EventId),
    PolicySelectd(PolicyPickList),
    AddressesChanged(Vec<GetAddress>, HashMap<ScriptBuf, u64>),
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct AddressesState {
    policy: Option<PolicyPickList>,
    policies: Vec<PolicyPickList>,
    addresses: Vec<GetAddress>,
    balances: HashMap<ScriptBuf, u64>,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl AddressesState {
    pub fn new(policy: Option<(EventId, Policy)>) -> Self {
        Self {
            policy: policy.map(|(policy_id, policy)| PolicyPickList {
                policy_id,
                name: policy.name,
            }),
            policies: Vec::new(),
            addresses: Vec::new(),
            balances: HashMap::new(),
            loading: false,
            loaded: false,
            error: None,
        }
    }
}

impl State for AddressesState {
    fn title(&self) -> String {
        String::from("Addresses")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
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
                         }| PolicyPickList {
                            policy_id,
                            name: policy.name,
                        },
                    )
                    .collect()
            },
            |p| AddressesMessage::LoadPolicies(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Addresses(msg) = message {
            match msg {
                AddressesMessage::LoadPolicies(policies) => {
                    self.policies = policies;
                    self.loading = false;
                    self.loaded = true;
                    if let Some(policy) = self.policy.as_ref() {
                        let policy_id = policy.policy_id;
                        return Command::perform(async {}, move |_| {
                            AddressesMessage::LoadAddresses(policy_id).into()
                        });
                    }
                }
                AddressesMessage::LoadAddresses(policy_id) => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move {
                            let addresses = client.get_addresses(policy_id).await?;
                            let balances = client.get_addresses_balances(policy_id).await?;
                            Ok::<
                                (Vec<GetAddress>, HashMap<ScriptBuf, u64>),
                                Box<dyn std::error::Error>,
                            >((addresses, balances))
                        },
                        |res| match res {
                            Ok((addresses, balances)) => {
                                AddressesMessage::AddressesChanged(addresses, balances).into()
                            }
                            Err(e) => AddressesMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                AddressesMessage::PolicySelectd(policy) => {
                    let policy_id = policy.policy_id;
                    self.policy = Some(policy);
                    return Command::perform(async {}, move |_| {
                        AddressesMessage::LoadAddresses(policy_id).into()
                    });
                }
                AddressesMessage::AddressesChanged(addresses, balances) => {
                    self.addresses = addresses;
                    self.balances = balances;
                }
                AddressesMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
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
                        .push(Text::new("Policy").view())
                        .push(
                            PickList::new(self.policies.clone(), self.policy.clone(), |policy| {
                                AddressesMessage::PolicySelectd(policy).into()
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
                        .spacing(5)
                        .max_width(400.0),
                )
                .push(Space::with_height(Length::Fixed(20.0)))
                .push(
                    Row::new()
                        .push(
                            Text::new("#")
                                .bold()
                                .big()
                                .horizontal_alignment(Horizontal::Center)
                                .width(Length::Fixed(80.0))
                                .view(),
                        )
                        .push(
                            Text::new("Address")
                                .bold()
                                .big()
                                .horizontal_alignment(Horizontal::Center)
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new("Label")
                                .bold()
                                .big()
                                .horizontal_alignment(Horizontal::Center)
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new("Balance")
                                .bold()
                                .big()
                                .horizontal_alignment(Horizontal::Center)
                                .width(Length::Fixed(125.0))
                                .view(),
                        )
                        .push(Space::with_width(Length::Fixed(40.0)))
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                )
                .push(rule::horizontal_bold());

            for (index, GetAddress { address, label }) in self.addresses.iter().enumerate() {
                let address = address.clone().assume_checked();
                let row = Row::new()
                    .push(
                        Text::new(index.to_string())
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Fixed(80.0))
                            .view(),
                    )
                    .push(
                        Text::new(address.to_string())
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(
                        Text::new(label.clone().unwrap_or_default())
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(
                        Text::new(format!(
                            "{} sat",
                            util::format::number(
                                self.balances
                                    .get(&address.script_pubkey())
                                    .copied()
                                    .unwrap_or_default()
                            )
                        ))
                        .horizontal_alignment(Horizontal::Center)
                        .width(Length::Fixed(125.0))
                        .view(),
                    )
                    .push(
                        Button::new()
                            .icon(CLIPBOARD)
                            .style(ButtonStyle::Bordered)
                            .on_press(Message::Clipboard(address.to_string()))
                            .width(Length::Fixed(40.0))
                            .view(),
                    )
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill);
                content = content.push(row).push(rule::horizontal());
            }

            content = content
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, false, false)
    }
}

impl From<AddressesState> for Box<dyn State> {
    fn from(s: AddressesState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<AddressesMessage> for Message {
    fn from(msg: AddressesMessage) -> Self {
        Self::Addresses(msg)
    }
}
