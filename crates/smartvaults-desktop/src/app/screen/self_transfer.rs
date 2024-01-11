// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Container, PickList, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::{Amount, FeeRate};
use smartvaults_sdk::nostr::EventId;
use smartvaults_sdk::types::GetProposal;
use smartvaults_sdk::util::format;

use crate::app::component::{Dashboard, FeeSelector, PolicyPickList};
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, NumericInput, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum SelfTransferMessage {
    LoadPolicies(Vec<PolicyPickList>),
    FromPolicySelectd(PolicyPickList),
    ToPolicySelectd(PolicyPickList),
    AmountChanged(Option<u64>),
    SendAllBtnPressed,
    FeeRateChanged(FeeRate),
    ErrorChanged(Option<String>),
    Review,
    EditProposal,
    SendProposal,
}

#[derive(Debug, Default)]
pub struct SelfTransferState {
    policies: Vec<PolicyPickList>,
    from_policy: Option<PolicyPickList>,
    to_policy: Option<PolicyPickList>,
    amount: Option<u64>,
    send_all: bool,
    fee_rate: FeeRate,
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
        let fee_rate = self.fee_rate;

        Command::perform(
            async move {
                let GetProposal { proposal_id, .. } = client
                    .self_transfer(
                        from_policy_id,
                        to_policy_id,
                        amount,
                        fee_rate,
                        None,
                        None,
                        false,
                    )
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
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                client
                    .vaults()
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|p| p.into())
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
                }
                SelfTransferMessage::FromPolicySelectd(policy) => {
                    self.from_policy = Some(policy);
                }
                SelfTransferMessage::ToPolicySelectd(policy) => {
                    self.to_policy = Some(policy);
                }
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
        let mut content = Column::new();

        if self.loaded {
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
                    .push(Row::new().push(Text::new(self.fee_rate.to_string()).view()))
                    .spacing(5)
                    .width(Length::Fill);

                let error = if let Some(error) = &self.error {
                    Row::new().push(Text::new(error).color(DARK_RED).view())
                } else {
                    Row::new()
                };

                content = content
                    .push(from_policy)
                    .push(to_policy)
                    .push(amount)
                    .push(priority)
                    .push(error)
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .text("Send proposal")
                            .width(Length::Fill)
                            .on_press(SelfTransferMessage::SendProposal.into())
                            .loading(self.loading)
                            .view(),
                    )
                    .push(
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .text("Back")
                            .width(Length::Fill)
                            .on_press(SelfTransferMessage::EditProposal.into())
                            .loading(self.loading)
                            .view(),
                    )
                    .max_width(400.0);
            } else {
                let from_policy_pick_list = Column::new()
                    .push(Text::new("From policy").view())
                    .push(
                        PickList::new(self.policies.clone(), self.from_policy.clone(), |policy| {
                            SelfTransferMessage::FromPolicySelectd(policy).into()
                        })
                        .width(Length::Fill)
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
                        .padding(10)
                        .placeholder(if self.policies.is_empty() {
                            "No policy availabe"
                        } else {
                            "Select a policy"
                        }),
                    )
                    .spacing(5);

                let send_all_btn = Button::new()
                    .style(ButtonStyle::Bordered)
                    .text("Max")
                    .width(Length::Fixed(50.0))
                    .on_press(SelfTransferMessage::SendAllBtnPressed.into())
                    .loading(self.loading || self.from_policy.is_none())
                    .view();

                let amount = if self.send_all {
                    TextInput::with_label("Amount (sat)", "Send all")
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

                let your_balance = if let Some(from_policy) = &self.from_policy {
                    Text::new(format!(
                        "Balance: {} sat",
                        format::number(from_policy.balance.trusted_spendable())
                    ))
                    .extra_light()
                    .small()
                    .width(Length::Fill)
                    .view()
                } else {
                    Text::new("").view()
                };

                let error = if let Some(error) = &self.error {
                    Row::new().push(Text::new(error).color(DARK_RED).view())
                } else {
                    Row::new()
                };

                let continue_btn = Button::new()
                    .text("Continue")
                    .width(Length::Fixed(400.0))
                    .on_press(SelfTransferMessage::Review.into())
                    .view();

                let details = Column::new()
                    .push(from_policy_pick_list)
                    .push(to_policy_pick_list)
                    .push(amount)
                    .push(your_balance)
                    .spacing(10)
                    .max_width(400);

                content = content
                    .push(
                        Column::new()
                            .push(Text::new("Self transfer").big().bold().view())
                            .push(
                                Text::new("Create a new spending proposal")
                                    .extra_light()
                                    .view(),
                            )
                            .spacing(10)
                            .width(Length::Fill),
                    )
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(
                        Row::new()
                            .push(details)
                            .push(rule::vertical())
                            .push(
                                FeeSelector::new(self.fee_rate, |f| {
                                    SelfTransferMessage::FeeRateChanged(f).into()
                                })
                                .current_mempool_fees(ctx.current_fees.clone())
                                .max_width(400.0),
                            )
                            .spacing(25)
                            .height(Length::Fixed(335.0)),
                    )
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(error)
                    .push(Space::with_height(Length::Fixed(5.0)))
                    .push(continue_btn)
                    .max_width(810.0);
            }
        }

        let content = Container::new(
            content
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20),
        )
        .width(Length::Fill)
        .center_x();

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
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
