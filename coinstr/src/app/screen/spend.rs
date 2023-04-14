// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bitcoin::{Address, Network};
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::proposal::SpendingProposal;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, NumericInput, Text, TextInput};
use crate::constants::APP_NAME;
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum SpendMessage {
    AddressChanged(String),
    AmountChanged(Option<u64>),
    MemoChanged(String),
    ErrorChanged(Option<String>),
    Review,
    EditProposal,
    SendProposal,
}

#[derive(Debug)]
pub struct SpendState {
    policy_id: EventId,
    to_address: String,
    amount: Option<u64>,
    memo: String,
    reviewing: bool,
    loading: bool,
    error: Option<String>,
}

impl SpendState {
    pub fn new(policy_id: EventId) -> Self {
        Self {
            policy_id,
            to_address: String::new(),
            amount: None,
            memo: String::new(),
            reviewing: false,
            loading: false,
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
                SpendMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                SpendMessage::Review => match self.amount {
                    Some(_) => match Address::from_str(&self.to_address) {
                        Ok(_) => {
                            self.error = None;
                            self.reviewing = true;
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    },
                    None => self.error = Some(String::from("Invalid amount")),
                },
                SpendMessage::EditProposal => self.reviewing = false,
                SpendMessage::SendProposal => {
                    self.loading = true;
                    match self.amount {
                        Some(amount) => match Address::from_str(&self.to_address) {
                            Ok(to_address) => {
                                let client = ctx.client.clone();
                                let cache = ctx.cache.clone();
                                let policy_id = self.policy_id;
                                let memo = self.memo.clone();

                                // TODO: get electrum endpoint from config file
                                let bitcoin_endpoint: &str = match ctx.coinstr.network() {
                                    Network::Bitcoin => "ssl://blockstream.info:700",
                                    Network::Testnet => "ssl://blockstream.info:993",
                                    _ => panic!("Endpoints not availabe for this network"),
                                };

                                return Command::perform(
                                    async move {
                                        let blockchain = ElectrumBlockchain::from(
                                            ElectrumClient::new(bitcoin_endpoint)?,
                                        );
                                        let (proposal_id, proposal) = client
                                            .spend(
                                                policy_id, to_address, amount, memo, blockchain,
                                                None,
                                            )
                                            .await?;
                                        cache
                                            .cache_proposal(
                                                proposal_id,
                                                policy_id,
                                                proposal.clone(),
                                            )
                                            .await;
                                        Ok::<(EventId, SpendingProposal), Box<dyn std::error::Error>>(
                                            (proposal_id, proposal),
                                        )
                                    },
                                    |res| match res {
                                        Ok((proposal_id, proposal)) => {
                                            Message::View(Stage::Proposal(proposal_id, proposal))
                                        }
                                        Err(e) => {
                                            SpendMessage::ErrorChanged(Some(e.to_string())).into()
                                        }
                                    },
                                );
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
        let content = if self.reviewing {
            let address = Column::new()
                .push(Row::new().push(Text::new("Address").bold().view()))
                .push(Row::new().push(Text::new(&self.to_address).view()))
                .spacing(5)
                .width(Length::Fill);

            let amount = Column::new()
                .push(Row::new().push(Text::new("Amount").bold().view()))
                .push(
                    Row::new().push(Text::new(self.amount.unwrap_or_default().to_string()).view()),
                )
                .spacing(5)
                .width(Length::Fill);

            let memo = Column::new()
                .push(Row::new().push(Text::new("Memo").bold().view()))
                .push(Row::new().push(Text::new(&self.memo).view()))
                .spacing(5)
                .width(Length::Fill);

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            let mut send_proposal_btn =
                button::primary("Send proposal").width(Length::Fixed(250.0));
            let mut back_btn = button::border("Back").width(Length::Fixed(250.0));

            if !self.loading {
                send_proposal_btn = send_proposal_btn.on_press(SpendMessage::SendProposal.into());
                back_btn = back_btn.on_press(SpendMessage::EditProposal.into());
            }

            Column::new()
                .push(address)
                .push(amount)
                .push(memo)
                .push(error)
                .push(Space::with_height(Length::Fixed(15.0)))
                .push(send_proposal_btn)
                .push(back_btn)
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400)
        } else {
            let address = TextInput::new("Address", &self.to_address)
                .on_input(|s| SpendMessage::AddressChanged(s).into())
                .placeholder("Address")
                .view();

            let amount = NumericInput::new("Amount", self.amount)
                .on_input(|s| SpendMessage::AmountChanged(s).into())
                .placeholder("Amount (sats)");

            let memo = TextInput::new("Memo", &self.memo)
                .on_input(|s| SpendMessage::MemoChanged(s).into())
                .placeholder("Memo")
                .view();

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            let continue_btn = button::primary("Continue").on_press(SpendMessage::Review.into());

            Column::new()
                .push(address)
                .push(amount)
                .push(memo)
                .push(error)
                .push(Space::with_height(Length::Fixed(15.0)))
                .push(continue_btn)
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400)
        };

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
