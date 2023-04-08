// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_core::bitcoin::{Address, Network};
use coinstr_core::nostr_sdk::EventId;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{button, NumericInput, Text, TextInput};
use crate::constants::APP_NAME;
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum SpendMessage {
    AddressChanged(String),
    AmountChanged(Option<u64>),
    MemoChanged(String),
    ErrorChanged(Option<String>),
    SendProposal,
}

#[derive(Debug)]
pub struct SpendState {
    policy_id: EventId,
    to_address: String,
    amount: Option<u64>,
    memo: String,
    error: Option<String>,
}

impl SpendState {
    pub fn new(policy_id: EventId) -> Self {
        Self {
            policy_id,
            to_address: String::new(),
            amount: None,
            memo: String::new(),
            error: None,
        }
    }
}

impl State for SpendState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Send")
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Spend(msg) = message {
            match msg {
                SpendMessage::AddressChanged(value) => self.to_address = value,
                SpendMessage::AmountChanged(value) => self.amount = value,
                SpendMessage::MemoChanged(value) => self.memo = value,
                SpendMessage::ErrorChanged(error) => self.error = error,
                SpendMessage::SendProposal => {
                    #[allow(unused_variables)]
                    match self.amount {
                        Some(amount) => match Address::from_str(&self.to_address) {
                            Ok(to_address) => {
                                let client = ctx.client.clone();
                                let policy_id = self.policy_id;
                                let memo = self.memo.clone();

                                // TODO: get electrum endpoint from config file
                                let bitcoin_endpoint: &str = match ctx.coinstr.network() {
                                    Network::Bitcoin => "ssl://blockstream.info:700",
                                    Network::Testnet => "ssl://blockstream.info:993",
                                    _ => panic!("Endpoints not availabe for this network"),
                                };

                                // TODO: send proposal
                            }
                            Err(e) => self.error = Some(e.to_string()),
                        },
                        None => self.error = Some(String::from("Invalid amount")),
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let address = TextInput::new("Address", &self.to_address, |s| {
            SpendMessage::AddressChanged(s).into()
        })
        .placeholder("Address")
        .view();

        let amount = NumericInput::new("Amount", self.amount, |s| {
            SpendMessage::AmountChanged(s).into()
        })
        .placeholder("Amount (sats)");

        let memo = TextInput::new("Memo", &self.memo, |s| SpendMessage::MemoChanged(s).into())
            .placeholder("Memo")
            .view();

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let send_porposal_btn =
            button::primary("Send proposal").on_press(SpendMessage::SendProposal.into());

        let content = Column::new()
            .push(address)
            .push(amount)
            .push(memo)
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(send_porposal_btn)
            .align_items(Alignment::Center)
            .spacing(10)
            .padding(20)
            .max_width(400);

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<SpendState> for Box<dyn State> {
    fn from(s: SpendState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SpendMessage> for Message {
    fn from(msg: SpendMessage) -> Self {
        Self::Spend(msg)
    }
}
