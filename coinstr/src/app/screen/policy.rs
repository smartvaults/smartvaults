// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::cmp::Ordering;

use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bdk::{Balance, SyncOptions, TransactionDetails};
use coinstr_core::bitcoin::Network;
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::policy::Policy;
use coinstr_core::util::{self, format};
use iced::alignment::Horizontal;
use iced::widget::{Column, Container, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{button, rule, Text};
use crate::constants::APP_NAME;
use crate::theme::icon::{ARROW_DOWN, ARROW_UP};

#[derive(Debug, Clone)]
pub enum PolicyMessage {
    Send,
    Deposit,
    WalledSynced(Balance, Vec<TransactionDetails>),
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

    // TODO: reload automatically balance every 60 secs

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
                PolicyMessage::Send => (),
                PolicyMessage::Deposit => (),
                PolicyMessage::WalledSynced(balance, txs) => {
                    self.balance = balance;
                    self.transactions = txs;
                    self.loading = false;
                    self.loaded = true;
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

        let pending_balance =
            self.balance.untrusted_pending + self.balance.trusted_pending + self.balance.immature;

        let row = Row::new()
            .push(
                Row::new()
                    .push(send_btn)
                    .push(deposit_btn)
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(
                Container::new(
                    Row::new()
                        .push(
                            Column::new()
                                .push(
                                    Text::new(format::number(self.balance.confirmed))
                                        .size(45)
                                        .view(),
                                )
                                .push(if pending_balance > 0 {
                                    Text::new(format::number(pending_balance)).size(25).view()
                                } else {
                                    Text::new("").size(25).view()
                                })
                                .align_items(Alignment::End),
                        )
                        .push(
                            Column::new()
                                .push(Space::with_height(Length::Fixed(8.0)))
                                .push(Text::new("sats").size(35).view())
                                .push(Space::with_height(Length::Fixed(29.0)))
                                .align_items(Alignment::End),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .align_x(Horizontal::Right),
            )
            .width(Length::Fill)
            .align_items(Alignment::Center);
        content = content
            .push(row)
            .push(Space::with_height(Length::Fixed(20.0)));

        let mut transactions = Column::new().spacing(10);

        if self.transactions.is_empty() {
            transactions = transactions.push(Text::new("No transactions").view());
        } else {
            for tx in self.transactions.iter().take(10) {
                let unconfirmed = match &tx.confirmation_time {
                    Some(block_time) => format!("block height: {}", block_time.height),
                    None => String::from(" - unconfirmed"),
                };
                let text = match tx.received.cmp(&tx.sent) {
                    Ordering::Greater => Text::new(format!(
                        "Received {} sats{unconfirmed}",
                        format::number(tx.received - tx.sent)
                    )),
                    Ordering::Less => {
                        let fee = match tx.fee {
                            Some(fee) => format!(" (fee: {} sats)", format::number(fee)),
                            None => String::new(),
                        };
                        Text::new(format!(
                            "Sent {} sats{fee}{unconfirmed}",
                            format::number(tx.sent - tx.received)
                        ))
                    }
                    Ordering::Equal => Text::new(format!("null{unconfirmed}")),
                };
                transactions = transactions.push(text.view()).push(rule::horizontal());
            }
        }

        let row = Row::new()
            .push(
                Column::new()
                    .push(Text::new("Latest transactions").bigger().view())
                    .push(rule::horizontal_bold())
                    .push(transactions)
                    .width(Length::Fill),
            )
            /* .push(Space::with_width(Length::Fixed(10.0)))
            .push(
                Column::new()
                    .push(Text::new("Descriptor").bigger().view())
                    .push(rule::horizontal_bold())
                    .push(Text::new(self.policy.descriptor.to_string()).view())
                    .width(Length::Fill),
            ) */;
        content = content.push(row);

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
