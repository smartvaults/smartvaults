// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::time::Duration;

use coinstr_sdk::core::bdk::Balance;
use coinstr_sdk::core::proposal::Proposal;
use coinstr_sdk::db::store::Transactions;
use coinstr_sdk::nostr::EventId;
use iced::widget::{Column, Space};
use iced::{time, Command, Element, Length, Subscription};

use crate::app::component::{Balances, Dashboard, PendingProposalsList, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::Text;
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum DashboardMessage {
    Send,
    Deposit,
    Load(
        Option<Balance>,
        BTreeMap<EventId, (EventId, Proposal)>,
        Option<Transactions>,
    ),
    Reload,
}

#[derive(Debug, Default)]
pub struct DashboardState {
    loading: bool,
    loaded: bool,
    balance: Option<Balance>,
    proposals: BTreeMap<EventId, (EventId, Proposal)>,
    transactions: Option<Transactions>,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            loading: false,
            loaded: false,
            balance: None,
            proposals: BTreeMap::new(),
            transactions: None,
        }
    }
}

impl State for DashboardState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Dashboard")
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(10)).map(|_| DashboardMessage::Reload.into())
        ])
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        self.loading = true;
        Command::perform(
            async move {
                let (balance, synced) = client.get_total_balance().unwrap();
                let txs = client.get_all_transactions().unwrap();
                let proposals = client.get_proposals().unwrap();

                if !synced {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }

                (balance, proposals, txs, synced)
            },
            |(balance, proposals, txs, synced)| {
                if synced {
                    DashboardMessage::Load(Some(balance), proposals, Some(txs)).into()
                } else {
                    DashboardMessage::Reload.into()
                }
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Dashboard(msg) = message {
            match msg {
                DashboardMessage::Send => {
                    return Command::perform(async {}, |_| Message::View(Stage::Spend(None)));
                }
                DashboardMessage::Deposit => {
                    return Command::perform(async {}, |_| Message::View(Stage::Receive(None)))
                }
                DashboardMessage::Load(balance, proposals, txs) => {
                    self.balance = balance;
                    self.proposals = proposals;
                    self.transactions = txs;
                    self.loading = false;
                    self.loaded = true;
                }
                DashboardMessage::Reload => {
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

        if self.loaded {
            center_y = false;
            center_x = false;

            content = content
                .push(Space::with_height(Length::Fixed(50.0)))
                .push(
                    Balances::new(self.balance.clone())
                        .bigger()
                        .on_send(DashboardMessage::Send.into())
                        .on_deposit(DashboardMessage::Deposit.into())
                        .view(),
                )
                .push(Space::with_height(Length::Fixed(20.0)))
                .push(Text::new("Pending proposals").bold().size(25).view())
                .push(Space::with_height(Length::Fixed(10.0)))
                .push(
                    PendingProposalsList::new(self.proposals.clone())
                        .take(3)
                        .view(),
                )
                .push(Space::with_height(Length::Fixed(20.0)))
                .push(Text::new("Transactions").bold().size(25).view())
                .push(Space::with_height(Length::Fixed(10.0)))
                .push(
                    TransactionsList::new(self.transactions.clone())
                        .take(3)
                        .view(),
                );
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, center_x, center_y)
    }
}

impl From<DashboardState> for Box<dyn State> {
    fn from(s: DashboardState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<DashboardMessage> for Message {
    fn from(msg: DashboardMessage) -> Self {
        Self::Dashboard(msg)
    }
}
