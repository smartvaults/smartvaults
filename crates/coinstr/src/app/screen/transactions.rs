// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::db::model::GetTransaction;
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util;
use iced::widget::{Column, Space};
use iced::{Command, Element, Length};

use crate::app::component::{Dashboard, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::Text;

#[derive(Debug, Clone)]
pub enum TransactionsMessage {
    LoadTxs(Vec<GetTransaction>),
    Reload,
}

#[derive(Debug)]
pub struct TransactionsState {
    loading: bool,
    loaded: bool,
    policy_id: Option<EventId>,
    transactions: Vec<GetTransaction>,
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
        String::from("Transactions")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        let policy_id = self.policy_id;
        self.loading = true;
        Command::perform(
            async move {
                match policy_id {
                    Some(policy_id) => client.get_txs(policy_id).await.ok(),
                    None => client.get_all_transactions().await.ok(),
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

        if self.loaded {
            let title = match self.policy_id {
                Some(policy_id) => {
                    format!("Transactions of policy #{}", util::cut_event_id(policy_id))
                }
                None => String::from("Transactions"),
            };
            content = content
                .push(Text::new(title).size(40).bold().view())
                .push(Space::with_height(Length::Fixed(40.0)))
                .push(TransactionsList::new(self.transactions.clone()).view(ctx));
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, false, false)
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
