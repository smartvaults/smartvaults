// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Space};
use iced::{Command, Element, Length};
use smartvaults_sdk::core::bdk::wallet::Balance;
use smartvaults_sdk::types::{GetProposal, GetTransaction};

use crate::app::component::{Balances, Dashboard, PendingProposalsList, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::Text;

#[derive(Debug, Clone)]
pub enum DashboardMessage {
    Send,
    Deposit,
    Load(Balance, Vec<GetProposal>, Vec<GetTransaction>),
    Reload,
}

#[derive(Debug, Default)]
pub struct DashboardState {
    loading: bool,
    loaded: bool,
    balance: Balance,
    proposals: Vec<GetProposal>,
    transactions: Vec<GetTransaction>,
}

impl DashboardState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for DashboardState {
    fn title(&self) -> String {
        String::from("Dashboard")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        self.loading = true;
        Command::perform(
            async move {
                let balance = client.get_total_balance().await.unwrap();
                let txs = client.get_all_transactions().await.unwrap();
                let proposals = client.get_proposals().await.unwrap();

                (balance, proposals, txs)
            },
            |(balance, proposals, txs)| DashboardMessage::Load(balance, proposals, txs).into(),
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
                .push(
                    Balances::new(self.balance.clone())
                        .bigger()
                        .hide(ctx.hide_balances)
                        .on_send(DashboardMessage::Send.into())
                        .on_deposit(DashboardMessage::Deposit.into())
                        .view(),
                )
                .push(Space::with_height(Length::Fixed(20.0)))
                .push(Text::new("Pending proposals").bold().big().view())
                .push(Space::with_height(Length::Fixed(10.0)))
                .push(
                    PendingProposalsList::new(self.proposals.clone())
                        .take(3)
                        .view(),
                )
                .push(Space::with_height(Length::Fixed(20.0)))
                .push(Text::new("Transactions").bold().big().view())
                .push(Space::with_height(Length::Fixed(10.0)))
                .push(
                    TransactionsList::new(self.transactions.clone())
                        .take(3)
                        .view(ctx),
                );
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, center_x, center_y)
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
