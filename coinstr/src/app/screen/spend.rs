// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bitcoin::{Address, Network};
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::FeeRate;
use iced::widget::{Column, Radio, Row, Space};
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
    FeeRateChanged(FeeRate),
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
    fee_rate: FeeRate,
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
            fee_rate: FeeRate::Medium,
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
                SpendMessage::FeeRateChanged(fee_rate) => self.fee_rate = fee_rate,
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
                                let fee_rate = self.fee_rate;

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
                                                policy_id, to_address, amount, memo, fee_rate,
                                                blockchain, None,
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

            let priority = Column::new()
                .push(Row::new().push(Text::new("Priority").bold().view()))
                .push(
                    Row::new().push(
                        Text::new(match self.fee_rate {
                            FeeRate::High => "High (10 - 20 minutes)".to_string(),
                            FeeRate::Medium => "Medium (20 - 60 minutes)".to_string(),
                            FeeRate::Low => "Low (1 - 2 hours)".to_string(),
                            FeeRate::Custom(target) => format!("Custom ({target} blocks)"),
                        })
                        .view(),
                    ),
                )
                .spacing(5)
                .width(Length::Fill);

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            let mut send_proposal_btn = button::primary("Send proposal").width(Length::Fill);
            let mut back_btn = button::border("Back").width(Length::Fill);

            if !self.loading {
                send_proposal_btn = send_proposal_btn.on_press(SpendMessage::SendProposal.into());
                back_btn = back_btn.on_press(SpendMessage::EditProposal.into());
            }

            Column::new()
                .push(address)
                .push(amount)
                .push(memo)
                .push(priority)
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

            let amount = NumericInput::new("Amount (sats)", self.amount)
                .on_input(|s| SpendMessage::AmountChanged(s).into())
                .placeholder("Amount");

            let memo = TextInput::new("Memo", &self.memo)
                .on_input(|s| SpendMessage::MemoChanged(s).into())
                .placeholder("Memo")
                .view();

            let fee_high_priority = Row::new()
                .push(Radio::new(
                    "",
                    FeeRate::High,
                    Some(self.fee_rate),
                    |fee_rate| SpendMessage::FeeRateChanged(fee_rate).into(),
                ))
                .push(
                    Column::new()
                        .push(Text::new("High").view())
                        .push(Text::new("10 - 20 minues").extra_light().size(18).view())
                        .spacing(5),
                )
                .align_items(Alignment::Center)
                .width(Length::Fill);

            let fee_medium_priority = Row::new()
                .push(Radio::new(
                    "",
                    FeeRate::Medium,
                    Some(self.fee_rate),
                    |fee_rate| SpendMessage::FeeRateChanged(fee_rate).into(),
                ))
                .push(
                    Column::new()
                        .push(Text::new("Medium").view())
                        .push(Text::new("20 - 60 minues").extra_light().size(18).view())
                        .spacing(5),
                )
                .align_items(Alignment::Center)
                .width(Length::Fill);

            let fee_low_priority = Row::new()
                .push(Radio::new(
                    "",
                    FeeRate::Low,
                    Some(self.fee_rate),
                    |fee_rate| SpendMessage::FeeRateChanged(fee_rate).into(),
                ))
                .push(
                    Column::new()
                        .push(Text::new("Low").view())
                        .push(Text::new("1 - 2 hours").extra_light().size(18).view())
                        .spacing(5),
                )
                .align_items(Alignment::Center)
                .width(Length::Fill);

            let fee_selector = Column::new()
                .push(Text::new("Priority & arrival time").view())
                .push(
                    Column::new()
                        .push(fee_high_priority)
                        .push(fee_medium_priority)
                        .push(fee_low_priority)
                        .spacing(10),
                )
                .spacing(5);

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            let continue_btn = button::primary("Continue")
                .width(Length::Fill)
                .on_press(SpendMessage::Review.into());

            Column::new()
                .push(
                    Column::new()
                        .push(Text::new("Send").size(24).bold().view())
                        .push(
                            Text::new("Create a new spending proposal")
                                .extra_light()
                                .view(),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(address)
                .push(amount)
                .push(memo)
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(fee_selector)
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(error)
                .push(Space::with_height(Length::Fixed(5.0)))
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
