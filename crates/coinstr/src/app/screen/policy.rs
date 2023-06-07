// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bdk::Balance;
use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::db::store::Transactions;
use coinstr_sdk::nostr::{EventId, Timestamp};
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::{Balances, Dashboard, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::constants::APP_NAME;
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
        Option<Balance>,
        Option<Transactions>,
        Option<Timestamp>,
    ),
    ErrorChanged(Option<String>),
    Reload,
    Rebroadcast,
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
    error: Option<String>,
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
            error: None,
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

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        let policy_id = self.policy_id;
        self.loading = true;
        Command::perform(
            async move { client.db.policy_with_details(policy_id) },
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
                                client.delete_policy_by_id(policy_id, None).await?;
                                Ok::<(), Box<dyn std::error::Error>>(())
                            },
                            |res| match res {
                                Ok(_) => Message::View(Stage::Policies),
                                Err(e) => PolicyMessage::ErrorChanged(Some(e.to_string())).into(),
                            },
                        );
                    }
                }
                PolicyMessage::LoadPolicy(policy, balance, list, last_sync) => {
                    self.policy = Some(policy);
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
                PolicyMessage::Rebroadcast => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.rebroadcast_all_events().await },
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

        let mut center_y = true;
        let mut center_x = true;

        if let Some(last_sync) = self.last_sync {
            center_y = false;
            center_x = false;

            let title = format!("Policy #{}", util::cut_event_id(self.policy_id));
            content = content
                .push(Text::new(title).size(40).bold().view())
                .push(Space::with_height(Length::Fixed(30.0)));

            let mut rebroadcast_btn = button::border_only_icon(GLOBE).width(Length::Fixed(40.0));

            if !self.loading {
                rebroadcast_btn = rebroadcast_btn.on_press(PolicyMessage::Rebroadcast.into());
            }

            let mut delete_btn = button::danger_border_only_icon(TRASH).width(Length::Fixed(40.0));

            if !self.loading {
                delete_btn = delete_btn.on_press(PolicyMessage::Delete.into());
            }

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
                                .push(
                                    Row::new()
                                        .push(
                                            button::border_only_icon(CLIPBOARD)
                                                .on_press(Message::Clipboard(
                                                    self.policy_id.to_string(),
                                                ))
                                                .width(Length::Fixed(40.0)),
                                        )
                                        .push(
                                            button::border_only_icon(PATCH_CHECK)
                                                .on_press(PolicyMessage::NewProofOfReserve.into())
                                                .width(Length::Fixed(40.0)),
                                        )
                                        .push(
                                            button::border_only_icon(SAVE)
                                                .on_press(PolicyMessage::SavePolicyBackup.into())
                                                .width(Length::Fixed(40.0)),
                                        )
                                        .push(rebroadcast_btn)
                                        .push(delete_btn)
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
                })
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
