// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::time::Duration;

use coinstr_core::bdk::{Balance, TransactionDetails};
use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Space};
use iced::{time, Command, Element, Length, Subscription};

use crate::app::component::{Balances, Dashboard, TransactionsList};
use crate::app::{Context, Message, State};
use crate::component::Text;
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum DashboardMessage {
    WalletsSynced(Balance, Vec<TransactionDetails>),
    Reload,
}

#[derive(Debug, Default)]
pub struct DashboardState {
    loading: bool,
    loaded: bool,
    balance: Balance,
    transactions: Vec<TransactionDetails>,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            loading: false,
            loaded: false,
            balance: Balance::default(),
            transactions: Vec::new(),
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
        let cache = ctx.cache.clone();
        self.loading = true;
        Command::perform(
            async move {
                let balance = cache.get_total_balance().await.unwrap();
                let txs = cache.get_all_transactions().await.unwrap();
                (balance, txs)
            },
            |(balance, txs)| DashboardMessage::WalletsSynced(balance, txs).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Dashboard(msg) = message {
            match msg {
                DashboardMessage::WalletsSynced(balance, txs) => {
                    self.balance = balance;
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
        let content = Column::new()
            .push(Text::new("Dashboard").size(40).bold().view())
            .push(Space::with_height(Length::Fixed(40.0)))
            .push(
                Container::new(Balances::new(self.balance.clone()).view())
                    .align_x(Horizontal::Right),
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                TransactionsList::new(self.transactions.clone())
                    .take(10)
                    .view(),
            )
            .spacing(10)
            .padding(20);

        Dashboard::new().view(ctx, content, false, false)
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
