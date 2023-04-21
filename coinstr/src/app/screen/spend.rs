// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;
use std::str::FromStr;

use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bitcoin::{Address, Network};
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::policy::Policy;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::{util, FeeRate};
use iced::widget::{Column, PickList, Radio, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, NumericInput, Text, TextInput};
use crate::constants::APP_NAME;
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone, Eq)]
pub struct PolicyPicLisk {
    pub policy_id: EventId,
    pub name: String,
}

impl PartialEq for PolicyPicLisk {
    fn eq(&self, other: &Self) -> bool {
        self.policy_id == other.policy_id
    }
}

impl fmt::Display for PolicyPicLisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - #{}", self.name, util::cut_event_id(self.policy_id))
    }
}

#[derive(Debug, Clone)]
pub enum SpendMessage {
    LoadPolicies(Vec<PolicyPicLisk>),
    PolicySelectd(PolicyPicLisk),
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
    policy: Option<PolicyPicLisk>,
    policies: Vec<PolicyPicLisk>,
    to_address: String,
    amount: Option<u64>,
    memo: String,
    fee_rate: FeeRate,
    reviewing: bool,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl SpendState {
    pub fn new(policy: Option<(EventId, Policy)>) -> Self {
        Self {
            policy: policy.map(|(policy_id, policy)| PolicyPicLisk {
                policy_id,
                name: policy.name,
            }),
            policies: Vec::new(),
            to_address: String::new(),
            amount: None,
            memo: String::new(),
            fee_rate: FeeRate::default(),
            reviewing: false,
            loading: false,
            loaded: false,
            error: None,
        }
    }
}

impl State for SpendState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Send")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let cache = ctx.cache.clone();
        Command::perform(
            async move {
                cache
                    .policies()
                    .await
                    .into_iter()
                    .map(|(policy_id, policy)| PolicyPicLisk {
                        policy_id,
                        name: policy.name,
                    })
                    .collect()
            },
            |p| SpendMessage::LoadPolicies(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::Spend(msg) = message {
            match msg {
                SpendMessage::LoadPolicies(policies) => {
                    self.policies = policies;
                    self.loading = false;
                    self.loaded = true;
                }
                SpendMessage::PolicySelectd(policy) => {
                    self.policy = Some(policy);
                }
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
                    match &self.policy {
                        Some(policy) => match self.amount {
                            Some(amount) => match Address::from_str(&self.to_address) {
                                Ok(to_address) => {
                                    self.loading = true;

                                    let client = ctx.client.clone();
                                    let cache = ctx.cache.clone();
                                    let policy_id = policy.policy_id;
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
                                                    policy_id,
                                                    to_address,
                                                    amount,
                                                    memo,
                                                    fee_rate,
                                                    &blockchain,
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
                                            Ok::<
                                                (EventId, SpendingProposal),
                                                Box<dyn std::error::Error>,
                                            >((
                                                proposal_id,
                                                proposal,
                                            ))
                                        },
                                        |res| match res {
                                            Ok((proposal_id, proposal)) => Message::View(
                                                Stage::Proposal(proposal_id, proposal),
                                            ),
                                            Err(e) => {
                                                SpendMessage::ErrorChanged(Some(e.to_string()))
                                                    .into()
                                            }
                                        },
                                    );
                                }
                                Err(e) => self.error = Some(e.to_string()),
                            },
                            None => self.error = Some(String::from("Invalid amount")),
                        },
                        None => self.error = Some(String::from("You must select a policy")),
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = if self.loaded {
            if self.reviewing {
                let policy = Column::new()
                    .push(Row::new().push(Text::new("Policy").bold().view()))
                    .push(Row::new().push(if let Some(policy) = &self.policy {
                        Text::new(policy.to_string()).view()
                    } else {
                        Text::new("Policy not selected").color(DARK_RED).view()
                    }))
                    .spacing(5)
                    .width(Length::Fill);

                let address = Column::new()
                    .push(Row::new().push(Text::new("Address").bold().view()))
                    .push(Row::new().push(Text::new(&self.to_address).view()))
                    .spacing(5)
                    .width(Length::Fill);

                let amount = Column::new()
                    .push(Row::new().push(Text::new("Amount").bold().view()))
                    .push(
                        Row::new()
                            .push(Text::new(self.amount.unwrap_or_default().to_string()).view()),
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
                    send_proposal_btn =
                        send_proposal_btn.on_press(SpendMessage::SendProposal.into());
                    back_btn = back_btn.on_press(SpendMessage::EditProposal.into());
                }

                Column::new()
                    .push(policy)
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
                let policy_pick_list = Column::new()
                    .push(Text::new("Policy").view())
                    .push(
                        PickList::new(self.policies.clone(), self.policy.clone(), |policy| {
                            SpendMessage::PolicySelectd(policy).into()
                        })
                        .width(Length::Fill)
                        .text_size(20)
                        .padding(10)
                        .placeholder(if self.policies.is_empty() {
                            "No policy availabe"
                        } else {
                            "Select a policy"
                        }),
                    )
                    .spacing(5);

                let address = TextInput::new("Address", &self.to_address)
                    .on_input(|s| SpendMessage::AddressChanged(s).into())
                    .placeholder("Address")
                    .view();

                let amount = NumericInput::new("Amount (sat)", self.amount)
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
                    .push(policy_pick_list)
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
            }
        } else {
            Column::new().push(Text::new("Loading...").view())
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
