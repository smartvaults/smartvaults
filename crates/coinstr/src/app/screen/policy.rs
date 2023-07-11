// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_sdk::core::bdk::Balance;
use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::core::signer::Signer;
use coinstr_sdk::core::Proposal;
use coinstr_sdk::db::store::Transactions;
use coinstr_sdk::nostr::{EventId, Timestamp};
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::{Balances, Dashboard, PendingProposalsList, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::color::RED;
use crate::theme::icon::{CLIPBOARD, GLOBE, PATCH_CHECK, SAVE, TRASH};

#[derive(Debug, Clone)]
pub enum PolicyMessage {
    Send,
    Deposit,
    NewProofOfReserve,
    SavePolicyBackup,
    Delete,
    LoadPolicy(
        Policy,
        BTreeMap<EventId, (EventId, Proposal)>,
        Option<Signer>,
        Option<Balance>,
        Option<Transactions>,
        Option<Timestamp>,
    ),
    ErrorChanged(Option<String>),
    Reload,
    RepublishSharedKeys,
}

#[derive(Debug)]
pub struct PolicyState {
    loading: bool,
    loaded: bool,
    policy_id: EventId,
    policy: Option<Policy>,
    proposals: BTreeMap<EventId, (EventId, Proposal)>,
    signer: Option<Signer>,
    balance: Option<Balance>,
    transactions: Option<Transactions>,
    last_sync: Option<Timestamp>,
    error: Option<String>,
}

impl PolicyState {
    pub fn new(policy_id: EventId) -> Self {
        Self {
            loading: false,
            loaded: false,
            policy_id,
            policy: None,
            proposals: BTreeMap::new(),
            signer: None,
            balance: None,
            transactions: None,
            last_sync: None,
            error: None,
        }
    }
}

impl State for PolicyState {
    fn title(&self) -> String {
        format!("Policy #{}", util::cut_event_id(self.policy_id))
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        let client = ctx.client.clone();
        let policy_id = self.policy_id;
        self.loading = true;
        Command::perform(
            async move {
                client.mark_notification_as_seen_by_id(policy_id).ok()?;
                let (policy, balance, list, last_sync) =
                    client.db.policy_with_details(policy_id)?;
                let proposals = client.get_proposals_by_policy_id(policy_id).ok()?;
                let signer = client
                    .search_signer_by_descriptor(policy.descriptor.clone())
                    .ok();
                Some((policy, proposals, signer, balance, list, last_sync))
            },
            |res| match res {
                Some((policy, proposals, signer, balance, list, last_sync)) => {
                    PolicyMessage::LoadPolicy(policy, proposals, signer, balance, list, last_sync)
                        .into()
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
                PolicyMessage::NewProofOfReserve => {
                    let policy_id = self.policy_id;
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::NewProof(Some((policy_id, policy)))),
                        None => Message::View(Stage::Policies),
                    });
                }
                PolicyMessage::SavePolicyBackup => {
                    let path = FileDialog::new()
                        .set_title("Export policy backup")
                        .set_file_name(&format!(
                            "policy-{}.json",
                            util::cut_event_id(self.policy_id)
                        ))
                        .save_file();

                    if let Some(path) = path {
                        match ctx.client.save_policy_backup(self.policy_id, &path) {
                            Ok(_) => log::info!("Exported policy backup to {}", path.display()),
                            Err(e) => log::error!("Impossible to create file: {e}"),
                        }
                    }
                }
                PolicyMessage::Delete => {
                    let client = ctx.client.clone();
                    let policy_id = self.policy_id;

                    let path = FileDialog::new()
                        .set_title("Export policy backup")
                        .set_file_name(&format!(
                            "policy-{}.json",
                            util::cut_event_id(self.policy_id)
                        ))
                        .save_file();

                    if let Some(path) = path {
                        self.loading = true;
                        return Command::perform(
                            async move {
                                client.save_policy_backup(policy_id, &path)?;
                                client.delete_policy_by_id(policy_id).await?;
                                Ok::<(), Box<dyn std::error::Error>>(())
                            },
                            |res| match res {
                                Ok(_) => Message::View(Stage::Policies),
                                Err(e) => PolicyMessage::ErrorChanged(Some(e.to_string())).into(),
                            },
                        );
                    }
                }
                PolicyMessage::LoadPolicy(policy, proposals, signer, balance, list, last_sync) => {
                    self.policy = Some(policy);
                    self.proposals = proposals;
                    self.signer = signer;
                    self.balance = balance;
                    self.transactions = list;
                    self.last_sync = last_sync;
                    self.loading = false;
                    self.loaded = true;
                }
                PolicyMessage::ErrorChanged(e) => {
                    self.loading = false;
                    self.error = e;
                }
                PolicyMessage::Reload => {
                    return self.load(ctx);
                }
                PolicyMessage::RepublishSharedKeys => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let policy_id = self.policy_id;
                    return Command::perform(
                        async move { client.republish_shared_key_for_policy(policy_id).await },
                        |res| match res {
                            Ok(_) => PolicyMessage::ErrorChanged(None).into(),
                            Err(e) => PolicyMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        if self.last_sync.is_some() {
            content = content
                .push(Space::with_height(Length::Fixed(20.0)))
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
                                        "Signer: {}",
                                        self.signer
                                            .as_ref()
                                            .map(|s| s.to_string())
                                            .unwrap_or_else(|| String::from("Unavailable"))
                                    ))
                                    .view(),
                                )
                                .push(
                                    Row::new()
                                        .push(
                                            Button::new()
                                                .style(ButtonStyle::Bordered)
                                                .icon(CLIPBOARD)
                                                .on_press(Message::Clipboard(
                                                    self.policy_id.to_string(),
                                                ))
                                                .width(Length::Fixed(40.0))
                                                .view(),
                                        )
                                        .push(
                                            Button::new()
                                                .style(ButtonStyle::Bordered)
                                                .icon(PATCH_CHECK)
                                                .on_press(PolicyMessage::NewProofOfReserve.into())
                                                .width(Length::Fixed(40.0))
                                                .view(),
                                        )
                                        .push(
                                            Button::new()
                                                .style(ButtonStyle::Bordered)
                                                .icon(SAVE)
                                                .on_press(PolicyMessage::SavePolicyBackup.into())
                                                .width(Length::Fixed(40.0))
                                                .view(),
                                        )
                                        .push(
                                            Button::new()
                                                .style(ButtonStyle::Bordered)
                                                .icon(GLOBE)
                                                .width(Length::Fixed(40.0))
                                                .on_press(PolicyMessage::RepublishSharedKeys.into())
                                                .loading(self.loading)
                                                .view(),
                                        )
                                        .push(
                                            Button::new()
                                                .style(ButtonStyle::BorderedDanger)
                                                .icon(TRASH)
                                                .width(Length::Fixed(40.0))
                                                .on_press(PolicyMessage::Delete.into())
                                                .loading(self.loading)
                                                .view(),
                                        )
                                        .spacing(10),
                                )
                                .spacing(10)
                                .max_width(300),
                        )
                        .push(Space::with_width(Length::Fixed(10.0)))
                        .push(
                            Column::new()
                                .push(rule::vertical())
                                .height(Length::Fixed(135.0))
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
                .push(if let Some(error) = &self.error {
                    Text::new(error).color(RED).view()
                } else {
                    Text::new("").view()
                });

            if !self.proposals.is_empty() {
                content = content
                    .push(Space::with_height(Length::Fixed(20.0)))
                    .push(Text::new("Pending proposals").bold().size(25).view())
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(
                        PendingProposalsList::new(self.proposals.clone())
                            .hide_policy_id()
                            .take(3)
                            .view(),
                    );
            }

            content = content
                .push(Space::with_height(Length::Fixed(20.0)))
                .push(Text::new("Transactions").bold().size(25).view())
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(
                    TransactionsList::new(self.transactions.clone())
                        .take(5)
                        .policy_id(self.policy_id)
                        .view(),
                );
        }

        Dashboard::new()
            .loaded(self.last_sync.is_some())
            .view(ctx, content, false, false)
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
