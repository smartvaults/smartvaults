// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::time::Duration;

use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bdk::{Balance, SyncOptions, TransactionDetails};
use coinstr_core::bitcoin::Network;
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::policy::Policy;
use coinstr_core::util;
use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Row, Space};
use iced::{time, Alignment, Command, Element, Length, Subscription};

use crate::app::component::{Balances, Dashboard, TransactionsList};
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text};
use crate::constants::APP_NAME;
use crate::theme::icon::{ARROW_DOWN, ARROW_UP};

#[derive(Debug, Clone)]
pub enum PolicyMessage {
    Send,
    Deposit,
    WalledSynced(Balance, Vec<TransactionDetails>),
    Reload,
}

#[derive(Debug)]
pub struct PolicyState {
    loading: bool,
    loaded: bool,
    policy_id: EventId,
    policy: Policy,
    balance: Balance,
    transactions: Vec<TransactionDetails>,
}

impl PolicyState {
    pub fn new(policy_id: EventId, policy: Policy) -> Self {
        Self {
            loading: false,
            loaded: false,
            policy_id,
            policy,
            balance: Balance::default(),
            transactions: Vec::new(),
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

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(60)).map(|_| PolicyMessage::Reload.into())
        ])
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        let descriptor = self.policy.descriptor.to_string();
        // TODO: get electrum endpoint from config file
        let bitcoin_endpoint: &str = match ctx.coinstr.network() {
            Network::Bitcoin => "ssl://blockstream.info:700",
            Network::Testnet => "ssl://blockstream.info:993",
            _ => panic!("Endpoints not availabe for this network"),
        };
        self.loading = true;
        Command::perform(
            async move {
                let wallet = client.wallet(descriptor).unwrap();
                let electrum_client = ElectrumClient::new(bitcoin_endpoint).unwrap();
                let blockchain = ElectrumBlockchain::from(electrum_client);
                wallet.sync(&blockchain, SyncOptions::default()).unwrap();
                let balance = wallet.get_balance().unwrap();
                let mut txs = wallet.list_transactions(false).unwrap();
                txs.sort_by(|a, b| {
                    b.confirmation_time
                        .as_ref()
                        .map(|t| t.height)
                        .cmp(&a.confirmation_time.as_ref().map(|t| t.height))
                });
                (balance, txs)
            },
            |(balance, txs)| PolicyMessage::WalledSynced(balance, txs).into(),
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
                    return Command::perform(async {}, move |_| {
                        Message::View(Stage::Spend(policy_id))
                    });
                }
                PolicyMessage::Deposit => (),
                PolicyMessage::WalledSynced(balance, txs) => {
                    self.balance = balance;
                    self.transactions = txs;
                    self.loading = false;
                    self.loaded = true;
                }
                PolicyMessage::Reload => {
                    return self.load(ctx);
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        let title = format!("Policy #{}", util::cut_event_id(self.policy_id));
        content = content.push(Text::new(title).size(40).bold().view());

        if self.loading {
            content = content
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(Text::new("Syncing with the timechain...").view())
                .push(Space::with_height(Length::Fixed(5.0)));
        } else {
            content = content.push(Space::with_height(Length::Fixed(40.0)));
        }

        /* content = content
        .push(Text::new(format!("Name: {}", &self.policy.name)).view())
        .push(Text::new(format!("Description: {}", &self.policy.description)).view()); */

        let send_btn = button::border_text_below_icon(ARROW_UP, "Send")
            .on_press(PolicyMessage::Send.into())
            .width(Length::Fixed(110.0));
        let deposit_btn = button::border_text_below_icon(ARROW_DOWN, "Receive")
            .on_press(PolicyMessage::Deposit.into())
            .width(Length::Fixed(110.0));

        content = content
            .push(
                Row::new()
                    .push(
                        Row::new()
                            .push(send_btn)
                            .push(deposit_btn)
                            .spacing(10)
                            .width(Length::Fill),
                    )
                    .push(
                        Container::new(Balances::new(self.balance.clone()).view())
                            .align_x(Horizontal::Right),
                    )
                    .width(Length::Fill)
                    .align_items(Alignment::Center),
            )
            .push(Space::with_height(Length::Fixed(20.0)))
            .push(
                TransactionsList::new(self.transactions.clone())
                    .take(10)
                    .view(),
            );

        Dashboard::new().view(ctx, content, false, false)
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
