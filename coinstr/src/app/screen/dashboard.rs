// Copyright (c) 2022-2023 Coinstr
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
    WalletsSynced(Option<Balance>, Option<Vec<TransactionDetails>>),
    Reload,
}

#[derive(Debug, Default)]
pub struct DashboardState {
    loading: bool,
    loaded: bool,
    balance: Option<Balance>,
    transactions: Option<Vec<TransactionDetails>>,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            loading: false,
            loaded: false,
            balance: None,
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
        let cache = ctx.cache.clone();
        self.loading = true;
        Command::perform(
            async move {
                let (balance, synced) = cache.get_total_balance().await.unwrap();
                let txs = cache.get_all_transactions().await.unwrap();

                if !synced {
                    tokio::time::sleep(Duration::from_secs(3)).await;
                }

                (balance, txs, synced)
            },
            |(balance, txs, synced)| {
                if synced {
                    DashboardMessage::WalletsSynced(Some(balance), Some(txs)).into()
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
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;
        let mut center_x = true;

        if self.loaded {
            center_y = false;
            center_x = false;

            content = content
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
