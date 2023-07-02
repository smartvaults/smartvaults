// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;

use coinstr_sdk::core::bdk::blockchain::{Blockchain, ElectrumBlockchain};
use coinstr_sdk::core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_sdk::core::bdk::Balance;
use coinstr_sdk::core::{Amount, FeeRate};
use coinstr_sdk::db::model::GetPolicyResult;
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util::{self, format};
use iced::widget::{Column, Container, PickList, Radio, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, NumericInput, Text, TextInput};
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
pub enum SelfTransferMessage {
    LoadPolicies(Vec<PolicyPicLisk>),
    FromPolicySelectd(PolicyPicLisk),
    ToPolicySelectd(PolicyPicLisk),
    LoadBalance(EventId),
    AmountChanged(Option<u64>),
    SendAllBtnPressed,
    FeeRateChanged(FeeRate),
    BalanceChanged(Option<Balance>),
    ErrorChanged(Option<String>),
    Review,
    EditProposal,
    SendProposal,
}

#[derive(Debug, Default)]
pub struct SelfTransferState {
    policies: Vec<PolicyPicLisk>,
    from_policy: Option<PolicyPicLisk>,
    to_policy: Option<PolicyPicLisk>,
    amount: Option<u64>,
    send_all: bool,
    fee_rate: FeeRate,
    balance: Option<Balance>,
    reviewing: bool,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl SelfTransferState {
    pub fn new() -> Self {
        Self::default()
    }

    fn spend(
        &mut self,
        ctx: &mut Context,
        from_policy_id: EventId,
        to_policy_id: EventId,
        amount: Amount,
    ) -> Command<Message> {
        self.loading = true;

        let client = ctx.client.clone();
        let target_blocks = self.fee_rate.target_blocks();

        Command::perform(
            async move {
                let endpoint: String = client.electrum_endpoint()?;
                let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
                let fee_rate = blockchain.estimate_fee(target_blocks)?;

                let (proposal_id, ..) = client
                    .self_transfer(from_policy_id, to_policy_id, amount, fee_rate)
                    .await?;
                Ok::<EventId, Box<dyn std::error::Error>>(proposal_id)
            },
            |res| match res {
                Ok(proposal_id) => Message::View(Stage::Proposal(proposal_id)),
                Err(e) => SelfTransferMessage::ErrorChanged(Some(e.to_string())).into(),
            },
        )
    }
}

impl State for SelfTransferState {
    fn title(&self) -> String {
        String::from("Self transfer")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                client
                    .get_policies()
                    .unwrap()
                    .into_iter()
                    .map(
                        |(policy_id, GetPolicyResult { policy, .. })| PolicyPicLisk {
                            policy_id,
                            name: policy.name,
                        },
                    )
                    .collect()
            },
            |p| SelfTransferMessage::LoadPolicies(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::SelfTransfer(msg) = message {
            match msg {
                SelfTransferMessage::LoadPolicies(policies) => {
                    self.policies = policies;
                    self.loading = false;
                    self.loaded = true;
                    if let Some(policy) = self.from_policy.as_ref() {
                        let policy_id = policy.policy_id;
                        return Command::perform(async {}, move |_| {
                            SelfTransferMessage::LoadBalance(policy_id).into()
                        });
                    }
                }
                SelfTransferMessage::FromPolicySelectd(policy) => {
                    let policy_id = policy.policy_id;
                    self.from_policy = Some(policy);
                    return Command::perform(async {}, move |_| {
                        SelfTransferMessage::LoadBalance(policy_id).into()
                    });
                }
                SelfTransferMessage::ToPolicySelectd(policy) => {
                    self.to_policy = Some(policy);
                }
                SelfTransferMessage::LoadBalance(policy_id) => {
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.get_balance(policy_id) },
                        |balance| SelfTransferMessage::BalanceChanged(balance).into(),
                    );
                }
                SelfTransferMessage::BalanceChanged(balance) => self.balance = balance,
                SelfTransferMessage::AmountChanged(value) => self.amount = value,
                SelfTransferMessage::SendAllBtnPressed => self.send_all = !self.send_all,
                SelfTransferMessage::FeeRateChanged(fee_rate) => self.fee_rate = fee_rate,
                SelfTransferMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                SelfTransferMessage::Review => match &self.from_policy {
                    Some(_) => match &self.to_policy {
                        Some(_) => {
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
                        None => self.error = Some(String::from("You must select a policy")),
                    },
                    None => self.error = Some(String::from("You must select a policy")),
                },
                SelfTransferMessage::EditProposal => self.reviewing = false,
                SelfTransferMessage::SendProposal => match &self.from_policy {
                    Some(from_policy) => {
                        let from_policy_id = from_policy.policy_id;
                        match &self.to_policy {
                            Some(to_policy) => {
                                let to_policy_id = to_policy.policy_id;
                                if self.send_all {
                                    return self.spend(
                                        ctx,
                                        from_policy_id,
                                        to_policy_id,
                                        Amount::Max,
                                    );
                                } else {
                                    match self.amount {
                                        Some(amount) => {
                                            return self.spend(
                                                ctx,
                                                from_policy_id,
                                                to_policy_id,
                                                Amount::Custom(amount),
                                            )
                                        }
                                        None => self.error = Some(String::from("Invalid amount")),
                                    };
                                }
                            }
                            None => self.error = Some(String::from("You must select a policy")),
                        }
                    }
                    None => self.error = Some(String::from("You must select a policy")),
                },
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let content = if self.loaded {
            if self.reviewing {
                let from_policy = Column::new()
                    .push(Row::new().push(Text::new("From policy").bold().view()))
                    .push(Row::new().push(if let Some(policy) = &self.from_policy {
                        Text::new(policy.to_string()).view()
                    } else {
                        Text::new("Policy not selected").color(DARK_RED).view()
                    }))
                    .spacing(5)
                    .width(Length::Fill);

                let to_policy = Column::new()
                    .push(Row::new().push(Text::new("To policy").bold().view()))
                    .push(Row::new().push(if let Some(policy) = &self.to_policy {
                        Text::new(policy.to_string()).view()
                    } else {
                        Text::new("Policy not selected").color(DARK_RED).view()
                    }))
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
                        send_proposal_btn.on_press(SelfTransferMessage::SendProposal.into());
                    back_btn = back_btn.on_press(SelfTransferMessage::EditProposal.into());
                }

                Column::new()
                    .push(from_policy)
                    .push(to_policy)
                    .push(amount)
                    .push(priority)
                    .push(error)
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(send_proposal_btn)
                    .push(back_btn)
            } else {
                let from_policy_pick_list = Column::new()
                    .push(Text::new("From policy").view())
                    .push(
                        PickList::new(self.policies.clone(), self.from_policy.clone(), |policy| {
                            SelfTransferMessage::FromPolicySelectd(policy).into()
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

                let to_policy_pick_list = Column::new()
                    .push(Text::new("To policy").view())
                    .push(
                        PickList::new(self.policies.clone(), self.to_policy.clone(), |policy| {
                            SelfTransferMessage::ToPolicySelectd(policy).into()
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

                let mut send_all_btn = button::border("Max").width(Length::Fixed(50.0));

                if self.from_policy.is_some() && self.to_policy.is_some() {
                    send_all_btn =
                        send_all_btn.on_press(SelfTransferMessage::SendAllBtnPressed.into());
                }

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
                                            .on_input(|s| {
                                                SelfTransferMessage::AmountChanged(s).into()
                                            })
                                            .placeholder("Amount"),
                                    )
                                    .width(Length::Fill),
                            )
                            .push(send_all_btn)
                            .align_items(Alignment::End)
                            .spacing(5),
                    )
                };

                let your_balance = if self.from_policy.is_some() {
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

                let fee_high_priority = Row::new()
                    .push(Radio::new(
                        "",
                        FeeRate::High,
                        Some(self.fee_rate),
                        |fee_rate| SelfTransferMessage::FeeRateChanged(fee_rate).into(),
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
                        |fee_rate| SelfTransferMessage::FeeRateChanged(fee_rate).into(),
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
                        |fee_rate| SelfTransferMessage::FeeRateChanged(fee_rate).into(),
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
                    .on_press(SelfTransferMessage::Review.into());

                Column::new()
                    .push(
                        Column::new()
                            .push(Text::new("Self transfer").size(24).bold().view())
                            .push(
                                Text::new("Create a new spending proposal")
                                    .extra_light()
                                    .view(),
                            )
                            .spacing(10)
                            .width(Length::Fill),
                    )
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(from_policy_pick_list)
                    .push(to_policy_pick_list)
                    .push(amount)
                    .push(your_balance)
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(fee_selector)
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(error)
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(continue_btn)
            }
        } else {
            Column::new().push(Text::new("Loading...").view())
        };

        let content = Container::new(
            content
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400),
        )
        .width(Length::Fill)
        .center_x();

        Dashboard::new().view(ctx, content, true, true)
    }
}

impl From<SelfTransferState> for Box<dyn State> {
    fn from(s: SelfTransferState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SelfTransferMessage> for Message {
    fn from(msg: SelfTransferMessage) -> Self {
        Self::SelfTransfer(msg)
    }
}
