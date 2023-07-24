// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;
use std::str::FromStr;

use coinstr_sdk::core::bdk::blockchain::{Blockchain, ElectrumBlockchain};
use coinstr_sdk::core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_sdk::core::bdk::Balance;
use coinstr_sdk::core::bitcoin::Address;
use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::core::{Amount, FeeRate};
use coinstr_sdk::db::model::{GetPolicy, GetProposal};
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util::{self, format};
use iced::widget::{Column, Container, PickList, Radio, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, ButtonStyle, NumericInput, Text, TextInput};
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
    LoadBalance(EventId),
    AddressChanged(String),
    AmountChanged(Option<u64>),
    SendAllBtnPressed,
    DescriptionChanged(String),
    FeeRateChanged(FeeRate),
    BalanceChanged(Option<Balance>),
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
    send_all: bool,
    description: String,
    fee_rate: FeeRate,
    balance: Option<Balance>,
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
            send_all: false,
            description: String::new(),
            fee_rate: FeeRate::default(),
            balance: None,
            reviewing: false,
            loading: false,
            loaded: false,
            error: None,
        }
    }

    fn spend(
        &mut self,
        ctx: &mut Context,
        policy_id: EventId,
        to_address: Address,
        amount: Amount,
    ) -> Command<Message> {
        self.loading = true;

        let client = ctx.client.clone();
        let description = self.description.clone();
        let target_blocks = self.fee_rate.target_blocks();

        Command::perform(
            async move {
                let endpoint: String = client.electrum_endpoint()?;
                let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
                let fee_rate = blockchain.estimate_fee(target_blocks)?;

                let GetProposal { proposal_id, .. } = client
                    .spend(policy_id, to_address, amount, description, fee_rate, None)
                    .await?;
                Ok::<EventId, Box<dyn std::error::Error>>(proposal_id)
            },
            |res| match res {
                Ok(proposal_id) => Message::View(Stage::Proposal(proposal_id)),
                Err(e) => SpendMessage::ErrorChanged(Some(e.to_string())).into(),
            },
        )
    }
}

impl State for SpendState {
    fn title(&self) -> String {
        String::from("Send")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                client
                    .get_policies()
                    .unwrap()
                    .into_iter()
                    .map(
                        |GetPolicy {
                             policy_id, policy, ..
                         }| PolicyPicLisk {
                            policy_id,
                            name: policy.name,
                        },
                    )
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
                    if let Some(policy) = self.policy.as_ref() {
                        let policy_id = policy.policy_id;
                        return Command::perform(async {}, move |_| {
                            SpendMessage::LoadBalance(policy_id).into()
                        });
                    }
                }
                SpendMessage::PolicySelectd(policy) => {
                    let policy_id = policy.policy_id;
                    self.policy = Some(policy);
                    return Command::perform(async {}, move |_| {
                        SpendMessage::LoadBalance(policy_id).into()
                    });
                }
                SpendMessage::LoadBalance(policy_id) => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.get_balance(policy_id) },
                        |balance| SpendMessage::BalanceChanged(balance).into(),
                    );
                }
                SpendMessage::BalanceChanged(balance) => self.balance = balance,
                SpendMessage::AddressChanged(value) => self.to_address = value,
                SpendMessage::AmountChanged(value) => self.amount = value,
                SpendMessage::SendAllBtnPressed => self.send_all = !self.send_all,
                SpendMessage::DescriptionChanged(value) => self.description = value,
                SpendMessage::FeeRateChanged(fee_rate) => self.fee_rate = fee_rate,
                SpendMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                SpendMessage::Review => match &self.policy {
                    Some(_) => match Address::from_str(&self.to_address) {
                        Ok(_) => {
                            if self.send_all {
                                self.error = None;
                                self.reviewing = true;
                            } else {
                                match self.amount {
                                    Some(_) => {
                                        self.error = None;
                                        self.reviewing = true;
                                    }
                                    None => self.error = Some(String::from("Invalid amount")),
                                };
                            }
                        }
                        Err(e) => self.error = Some(e.to_string()),
                    },
                    None => self.error = Some(String::from("You must select a policy")),
                },
                SpendMessage::EditProposal => self.reviewing = false,
                SpendMessage::SendProposal => match &self.policy {
                    Some(policy) => {
                        let policy_id = policy.policy_id;
                        match Address::from_str(&self.to_address) {
                            Ok(to_address) => {
                                if self.send_all {
                                    return self.spend(ctx, policy_id, to_address, Amount::Max);
                                } else {
                                    match self.amount {
                                        Some(amount) => {
                                            return self.spend(
                                                ctx,
                                                policy_id,
                                                to_address,
                                                Amount::Custom(amount),
                                            )
                                        }
                                        None => self.error = Some(String::from("Invalid amount")),
                                    };
                                }
                            }
                            Err(e) => self.error = Some(e.to_string()),
                        }
                    }
                    None => self.error = Some(String::from("You must select a policy")),
                },
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new();

        if self.loaded {
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
                        Row::new().push(
                            Text::new(if self.send_all {
                                String::from("Send all")
                            } else {
                                self.amount.unwrap_or_default().to_string()
                            })
                            .view(),
                        ),
                    )
                    .spacing(5)
                    .width(Length::Fill);

                let description = if !self.description.is_empty() {
                    Column::new()
                        .push(Row::new().push(Text::new("Description").bold().view()))
                        .push(Row::new().push(Text::new(&self.description).view()))
                        .spacing(5)
                        .width(Length::Fill)
                } else {
                    Column::new()
                };

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

                let send_proposal_btn = Button::new()
                    .text("Send proposal")
                    .width(Length::Fill)
                    .on_press(SpendMessage::SendProposal.into())
                    .loading(self.loading)
                    .view();
                let back_btn = Button::new()
                    .style(ButtonStyle::Bordered)
                    .text("Back")
                    .width(Length::Fill)
                    .on_press(SpendMessage::EditProposal.into())
                    .loading(self.loading)
                    .view();

                content = content
                    .push(policy)
                    .push(address)
                    .push(amount)
                    .push(description)
                    .push(priority)
                    .push(error)
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(send_proposal_btn)
                    .push(back_btn)
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

                let address = Column::new()
                    .push(
                        TextInput::new("Address", &self.to_address)
                            .on_input(|s| SpendMessage::AddressChanged(s).into())
                            .placeholder("Address")
                            .view(),
                    )
                    .push(
                        Text::new("Transfer to other policy")
                            .extra_light()
                            .smaller()
                            .on_press(Message::View(Stage::SelfTransfer))
                            .view(),
                    )
                    .spacing(5);

                let send_all_btn = Button::new()
                    .style(ButtonStyle::Bordered)
                    .text("Max")
                    .width(Length::Fixed(50.0))
                    .on_press(SpendMessage::SendAllBtnPressed.into())
                    .loading(self.policy.is_none())
                    .view();

                let amount = if self.send_all {
                    TextInput::new("Amount (sat)", "Send all")
                        .button(send_all_btn)
                        .view()
                } else {
                    Column::new().push(
                        Row::new()
                            .push(
                                Column::new()
                                    .push(
                                        NumericInput::new("Amount (sat)", self.amount)
                                            .on_input(|s| SpendMessage::AmountChanged(s).into())
                                            .placeholder("Amount"),
                                    )
                                    .width(Length::Fill),
                            )
                            .push(send_all_btn)
                            .align_items(Alignment::End)
                            .spacing(5),
                    )
                };

                let your_balance = if self.policy.is_some() {
                    Text::new(match &self.balance {
                        Some(balance) => {
                            format!("Balance: {} sat", format::number(balance.get_spendable()))
                        }
                        None => String::from("Loading..."),
                    })
                    .extra_light()
                    .smaller()
                    .width(Length::Fill)
                    .view()
                } else {
                    Text::new("").view()
                };

                let description = TextInput::new("Description", &self.description)
                    .on_input(|s| SpendMessage::DescriptionChanged(s).into())
                    .placeholder("Description")
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

                let continue_btn = Button::new()
                    .text("Continue")
                    .width(Length::Fill)
                    .on_press(SpendMessage::Review.into())
                    .view();

                content = content
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
                    .push(your_balance)
                    .push(description)
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(fee_selector)
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(error)
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(continue_btn);
            }
        }

        let content = Container::new(
            content
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400),
        )
        .width(Length::Fill)
        .center_x();

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
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
