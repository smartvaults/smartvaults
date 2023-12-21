// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeSet;

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;
use smartvaults_sdk::core::signer::Signer;
use smartvaults_sdk::nostr::EventId;
use smartvaults_sdk::types::{GetPolicy, GetProposal, GetTransaction};
use smartvaults_sdk::util;

pub mod add;
pub mod builder;
pub mod restore;
pub mod tree;
pub mod vaults;

use crate::app::component::{Activity, Balances, Dashboard};
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::color::RED;
use crate::theme::icon::{BINOCULARS, CLIPBOARD, GLOBE, PATCH_CHECK, SAVE, TRASH};

#[derive(Debug, Clone)]
pub enum VaultMessage {
    Send,
    Deposit,
    NewProofOfReserve,
    SavePolicyBackup,
    Delete,
    LoadPolicy(
        GetPolicy,
        Vec<GetProposal>,
        Option<Signer>,
        BTreeSet<GetTransaction>,
    ),
    ErrorChanged(Option<String>),
    Reload,
    RepublishSharedKeys,
}

#[derive(Debug)]
pub struct VaultState {
    loading: bool,
    loaded: bool,
    policy_id: EventId,
    policy: Option<GetPolicy>,
    proposals: Vec<GetProposal>,
    signer: Option<Signer>,
    transactions: BTreeSet<GetTransaction>,
    error: Option<String>,
}

impl VaultState {
    pub fn new(policy_id: EventId) -> Self {
        Self {
            loading: false,
            loaded: false,
            policy_id,
            policy: None,
            proposals: Vec::new(),
            signer: None,
            transactions: BTreeSet::new(),
            error: None,
        }
    }
}

impl State for VaultState {
    fn title(&self) -> String {
        format!("Vault #{}", util::cut_event_id(self.policy_id))
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
                let list = client.get_txs(policy_id).await.ok()?;
                let proposals = client.get_proposals_by_policy_id(policy_id).await.ok()?;
                let signer = client
                    .search_signer_by_descriptor(policy.policy.descriptor())
                    .await
                    .ok();
                Some((policy, proposals, signer, list))
            },
            |res| match res {
                Some((policy, proposals, signer, list)) => {
                    VaultMessage::LoadPolicy(policy, proposals, signer, list).into()
                }
                None => Message::View(Stage::Vaults),
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Policy(msg) = message {
            match msg {
                VaultMessage::Send => {
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::Spend(Some(policy))),
                        None => Message::View(Stage::Vaults),
                    });
                }
                VaultMessage::Deposit => {
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::Receive(Some(policy))),
                        None => Message::View(Stage::Vaults),
                    });
                }
                VaultMessage::NewProofOfReserve => {
                    let policy = self.policy.clone();
                    return Command::perform(async {}, move |_| match policy {
                        Some(policy) => Message::View(Stage::NewProof(Some(policy))),
                        None => Message::View(Stage::Vaults),
                    });
                }
                VaultMessage::SavePolicyBackup => {
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
                                Ok(_) => VaultMessage::Reload.into(),
                                Err(e) => VaultMessage::ErrorChanged(Some(e.to_string())).into(),
                            },
                        );
                    }
                }
                VaultMessage::Delete => {
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
                                Ok(_) => Message::View(Stage::Vaults),
                                Err(e) => VaultMessage::ErrorChanged(Some(e.to_string())).into(),
                            },
                        );
                    }
                }
                VaultMessage::LoadPolicy(policy, proposals, signer, list) => {
                    self.policy = Some(policy);
                    self.proposals = proposals;
                    self.signer = signer;
                    self.transactions = list;
                    self.loading = false;
                    self.loaded = true;
                }
                VaultMessage::ErrorChanged(e) => {
                    self.loading = false;
                    self.error = e;
                }
                VaultMessage::Reload => {
                    return self.load(ctx);
                }
                VaultMessage::RepublishSharedKeys => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let policy_id = self.policy_id;
                    return Command::perform(
                        async move { client.republish_shared_key_for_policy(policy_id).await },
                        |res| match res {
                            Ok(_) => VaultMessage::ErrorChanged(None).into(),
                            Err(e) => VaultMessage::ErrorChanged(Some(e.to_string())).into(),
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
                                        Text::new(format!("Name: {}", policy.policy.name())).view(),
                                    )
                                    .push(
                                        Text::new(format!(
                                            "Description: {}",
                                            policy.policy.description()
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
                                                        VaultMessage::NewProofOfReserve.into(),
                                                    )
                                                    .width(Length::Fixed(40.0))
                                                    .view(),
                                            )
                                            .push(
                                                Button::new()
                                                    .style(ButtonStyle::Bordered)
                                                    .icon(SAVE)
                                                    .on_press(VaultMessage::SavePolicyBackup.into())
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
                                                        VaultMessage::RepublishSharedKeys.into(),
                                                    )
                                                    .loading(self.loading)
                                                    .view(),
                                            )
                                            .push(
                                                Button::new()
                                                    .style(ButtonStyle::BorderedDanger)
                                                    .icon(TRASH)
                                                    .width(Length::Fixed(40.0))
                                                    .on_press(VaultMessage::Delete.into())
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
                                    .on_send(VaultMessage::Send.into())
                                    .on_deposit(VaultMessage::Deposit.into())
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

                content = content
                    .push(Space::with_height(Length::Fixed(20.0)))
                    .push(Text::new("Activity").bold().big().view())
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(
                        Activity::new(self.proposals.clone(), self.transactions.clone())
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

impl From<VaultState> for Box<dyn State> {
    fn from(s: VaultState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<VaultMessage> for Message {
    fn from(msg: VaultMessage) -> Self {
        Self::Policy(msg)
    }
}
