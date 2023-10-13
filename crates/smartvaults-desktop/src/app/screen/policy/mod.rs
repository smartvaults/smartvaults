// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;
use smartvaults_sdk::core::signer::Signer;
use smartvaults_sdk::nostr::EventId;
use smartvaults_sdk::types::{GetPolicy, GetProposal, GetTransaction};
use smartvaults_sdk::util;

pub mod add;
pub mod builder;
pub mod policies;
pub mod restore;
pub mod tree;

use crate::app::component::{Balances, Dashboard, PendingProposalsList, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::color::RED;
use crate::theme::icon::{BINOCULARS, CLIPBOARD, GLOBE, PATCH_CHECK, SAVE, TRASH};

#[derive(Debug, Clone)]
pub enum PolicyMessage {
    Send,
    Deposit,
    NewProofOfReserve,
    SavePolicyBackup,
    Delete,
    LoadPolicy(
        GetPolicy,
        Vec<GetProposal>,
        Option<Signer>,
        Vec<GetTransaction>,
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
    policy: Option<GetPolicy>,
    proposals: Vec<GetProposal>,
    signer: Option<Signer>,
    transactions: Vec<GetTransaction>,
    error: Option<String>,
}

impl PolicyState {
    pub fn new(policy_id: EventId) -> Self {
        Self {
            loading: false,
            loaded: false,
            policy_id,
            policy: None,
            proposals: Vec::new(),
            signer: None,
            transactions: Vec::new(),
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
                let policy = client.get_policy_by_id(policy_id).await.ok()?;
                let list = client.get_txs(policy_id, true).await.ok()?;
                let proposals = client.get_proposals_by_policy_id(policy_id).await.ok()?;
                let signer = client
                    .search_signer_by_descriptor(policy.policy.descriptor.clone())
                    .await
                    .ok();
                Some((policy, proposals, signer, list))
            },
            |res| match res {
                Some((policy, proposals, signer, list)) => {
                    PolicyMessage::LoadPolicy(policy, proposals, signer, list).into()
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
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::Spend(Some(policy))),
                        None => Message::View(Stage::Policies),
                    });
                }
                PolicyMessage::Deposit => {
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::Receive(Some(policy))),
                        None => Message::View(Stage::Policies),
                    });
                }
                PolicyMessage::NewProofOfReserve => {
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::NewProof(Some(policy))),
                        None => Message::View(Stage::Policies),
                    });
                }
                PolicyMessage::SavePolicyBackup => {
                    let path = FileDialog::new()
                        .set_title("Export policy backup")
                        .set_file_name(format!(
                            "policy-{}.json",
                            util::cut_event_id(self.policy_id)
                        ))
                        .save_file();

                    if let Some(path) = path {
                        let policy_id = self.policy_id;
                        let client = ctx.client.clone();
                        return Command::perform(
                            async move { client.save_policy_backup(policy_id, path).await },
                            move |res| match res {
                                Ok(_) => PolicyMessage::Reload.into(),
                                Err(e) => PolicyMessage::ErrorChanged(Some(e.to_string())).into(),
                            },
                        );
                    }
                }
                PolicyMessage::Delete => {
                    let client = ctx.client.clone();
                    let policy_id = self.policy_id;

                    let path = FileDialog::new()
                        .set_title("Export policy backup")
                        .set_file_name(format!(
                            "policy-{}.json",
                            util::cut_event_id(self.policy_id)
                        ))
                        .save_file();

                    if let Some(path) = path {
                        self.loading = true;
                        return Command::perform(
                            async move {
                                client.save_policy_backup(policy_id, &path).await?;
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
                PolicyMessage::LoadPolicy(policy, proposals, signer, list) => {
                    self.policy = Some(policy);
                    self.proposals = proposals;
                    self.signer = signer;
                    self.transactions = list;
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

        let is_ready = self.policy.is_some() && self.policy.as_ref().map(|p| p.last_sync).is_some();

        if is_ready {
            if let Some(policy) = &self.policy {
                content = content
                    .push(Space::with_height(Length::Fixed(20.0)))
                    .push(
                        Row::new()
                            .push(
                                Column::new()
                                    .push(
                                        Text::new(format!("Name: {}", policy.policy.name.as_str()))
                                            .view(),
                                    )
                                    .push(
                                        Text::new(format!(
                                            "Description: {}",
                                            policy.policy.description.as_str()
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
                                                    .on_press(
                                                        PolicyMessage::NewProofOfReserve.into(),
                                                    )
                                                    .width(Length::Fixed(40.0))
                                                    .view(),
                                            )
                                            .push(
                                                Button::new()
                                                    .style(ButtonStyle::Bordered)
                                                    .icon(SAVE)
                                                    .on_press(
                                                        PolicyMessage::SavePolicyBackup.into(),
                                                    )
                                                    .width(Length::Fixed(40.0))
                                                    .view(),
                                            )
                                            .push(
                                                Button::new()
                                                    .style(ButtonStyle::Bordered)
                                                    .icon(BINOCULARS)
                                                    .width(Length::Fixed(40.0))
                                                    .on_press(Message::View(Stage::PolicyTree(
                                                        self.policy_id,
                                                    )))
                                                    .loading(self.loading)
                                                    .view(),
                                            )
                                            .push(
                                                Button::new()
                                                    .style(ButtonStyle::Bordered)
                                                    .icon(GLOBE)
                                                    .width(Length::Fixed(40.0))
                                                    .on_press(
                                                        PolicyMessage::RepublishSharedKeys.into(),
                                                    )
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
                                Balances::new(policy.balance.clone())
                                    .hide(ctx.hide_balances)
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
                        .push(Text::new("Pending proposals").bold().big().view())
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
                    .push(Text::new("Transactions").bold().big().view())
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(
                        TransactionsList::new(self.transactions.clone())
                            .take(5)
                            .policy_id(self.policy_id)
                            .hide_policy_id()
                            .view(ctx),
                    );
            }
        }

        Dashboard::new()
            .loaded(is_ready)
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
