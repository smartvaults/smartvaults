// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::time::Duration;

use coinstr_core::bdk::Balance;
use coinstr_core::nostr_sdk::{EventId, Timestamp};
use coinstr_core::policy::Policy;
use coinstr_core::util;
use iced::widget::{Column, Row, Space};
use iced::{time, Alignment, Command, Element, Length, Subscription};

use crate::app::cache::Transactions;
use crate::app::component::{Balances, Dashboard, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Text};
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum PolicyMessage {
    Send,
    Deposit,
    LoadPolicy(
        Policy,
        Option<Balance>,
        Option<Transactions>,
        Option<Timestamp>,
    ),
    Reload,
}

#[derive(Debug)]
pub struct PolicyState {
    loading: bool,
    loaded: bool,
    policy_id: EventId,
    policy: Option<Policy>,
    balance: Option<Balance>,
    transactions: Option<Transactions>,
    last_sync: Option<Timestamp>,
}

impl PolicyState {
    pub fn new(policy_id: EventId) -> Self {
        Self {
            loading: false,
            loaded: false,
            policy_id,
            policy: None,
            balance: None,
            transactions: None,
            last_sync: None,
        }
    }
}

impl State for PolicyState {
    fn title(&self) -> String {
        format!(
            "{APP_NAME} - Policy #{}",
            util::cut_event_id(self.policy_id)
        )
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(10)).map(|_| PolicyMessage::Reload.into())
        ])
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let cache = ctx.cache.clone();
        let policy_id = self.policy_id;
        self.loading = true;
        Command::perform(
            async move { cache.policy(policy_id).await },
            |res| match res {
                Some((policy, balance, list, last_sync)) => {
                    PolicyMessage::LoadPolicy(policy, balance, list, last_sync).into()
                }
                None => Message::View(Stage::Policies),
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Policy(msg) = message {
            match msg {
                PolicyMessage::Send => {
                    let policy_id = self.policy_id;
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::Spend(Some((policy_id, policy)))),
                        None => Message::View(Stage::Policies),
                    });
                }
                PolicyMessage::Deposit => {
                    let policy_id = self.policy_id;
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::Receive(Some((policy_id, policy)))),
                        None => Message::View(Stage::Policies),
                    });
                }
                PolicyMessage::LoadPolicy(policy, balance, list, last_sync) => {
                    self.policy = Some(policy);
                    self.balance = balance;
                    self.transactions = list;
                    self.last_sync = last_sync;
                    self.loading = false;
                    self.loaded = true;
                }
                PolicyMessage::Reload => {
                    return self.load(ctx);
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        let mut center_y = true;
        let mut center_x = true;

        if let Some(last_sync) = self.last_sync {
            center_y = false;
            center_x = false;

            let title = format!("Policy #{}", util::cut_event_id(self.policy_id));
            content = content
                .push(Text::new(title).size(40).bold().view())
                .push(Space::with_height(Length::Fixed(30.0)));

            content = content
                .push(
                    Row::new()
                        .push(
                            Column::new()
                                .push(
                                    Text::new(format!(
                                        "Name: {}",
                                        self.policy
                                            .as_ref()
                                            .map(|p| p.name.as_str())
                                            .unwrap_or("Unavailable")
                                    ))
                                    .view(),
                                )
                                .push(
                                    Text::new(format!(
                                        "Description: {}",
                                        self.policy
                                            .as_ref()
                                            .map(|p| p.description.as_str())
                                            .unwrap_or("Unavailable")
                                    ))
                                    .view(),
                                )
                                .push(
                                    Text::new(format!(
                                        "Last sync: {}",
                                        last_sync.to_human_datetime()
                                    ))
                                    .view(),
                                )
                                .spacing(10)
                                .max_width(300),
                        )
                        .push(Space::with_width(Length::Fixed(10.0)))
                        .push(
                            Column::new()
                                .push(rule::vertical())
                                .height(Length::Fixed(125.0))
                                .align_items(Alignment::Center),
                        )
                        .push(Space::with_width(Length::Fixed(10.0)))
                        .push(
                            Balances::new(self.balance.clone())
                                .on_send(PolicyMessage::Send.into())
                                .on_deposit(PolicyMessage::Deposit.into())
                                .view(),
                        )
                        .spacing(20)
                        .width(Length::Fill)
                        .align_items(Alignment::Center),
                )
                .push(Space::with_height(Length::Fixed(20.0)))
                .push(
                    TransactionsList::new(self.transactions.clone())
                        .take(5)
                        .policy_id(self.policy_id)
                        .view(),
                );
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, center_x, center_y)
    }
}

impl From<PolicyState> for Box<dyn State> {
    fn from(s: PolicyState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<PolicyMessage> for Message {
    fn from(msg: PolicyMessage) -> Self {
        Self::Policy(msg)
    }
}
