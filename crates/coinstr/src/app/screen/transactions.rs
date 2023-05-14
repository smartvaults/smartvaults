// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::time::Duration;

use coinstr_core::db::store::Transactions;
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::util;
use iced::widget::{Column, Space};
use iced::{time, Command, Element, Length, Subscription};

use crate::app::component::{Dashboard, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::Text;
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum TransactionsMessage {
    LoadTxs(Transactions),
    Reload,
}

#[derive(Debug)]
pub struct TransactionsState {
    loading: bool,
    loaded: bool,
    policy_id: Option<EventId>,
    transactions: Transactions,
}

impl TransactionsState {
    pub fn new(policy_id: Option<EventId>) -> Self {
        Self {
            loading: false,
            loaded: false,
            policy_id,
            transactions: Vec::new(),
        }
    }
}

impl State for TransactionsState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Transactions")
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(10)).map(|_| TransactionsMessage::Reload.into())
        ])
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        let policy_id = self.policy_id;
        self.loading = true;
        Command::perform(
            async move {
                match policy_id {
                    Some(policy_id) => client.get_transactions(policy_id),
                    None => client.get_all_transactions().ok(),
                }
            },
            |res| match res {
                Some(list) => TransactionsMessage::LoadTxs(list).into(),
                None => Message::View(Stage::Policies),
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Transactions(msg) = message {
            match msg {
                TransactionsMessage::LoadTxs(list) => {
                    self.transactions = list;
                    self.loading = false;
                    self.loaded = true;
                }
                TransactionsMessage::Reload => {
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

            let title = match self.policy_id {
                Some(policy_id) => format!("Policy #{}", util::cut_event_id(policy_id)),
                None => String::from("All policies"),
            };
            content = content
                .push(Text::new(title).size(40).bold().view())
                .push(Space::with_height(Length::Fixed(40.0)))
                .push(TransactionsList::new(Some(self.transactions.clone())).view());
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, center_x, center_y)
    }
}

impl From<TransactionsState> for Box<dyn State> {
    fn from(s: TransactionsState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<TransactionsMessage> for Message {
    fn from(msg: TransactionsMessage) -> Self {
        Self::Transactions(msg)
    }
}
