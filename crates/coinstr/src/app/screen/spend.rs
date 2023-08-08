// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};
use std::fmt;
use std::str::FromStr;

use coinstr_sdk::core::bdk::descriptor::policy::SatisfiableItem;
use coinstr_sdk::core::bdk::wallet::Balance;
use coinstr_sdk::core::bitcoin::{Address, OutPoint};
use coinstr_sdk::core::policy::Policy;
use coinstr_sdk::core::{Amount, FeeRate};
use coinstr_sdk::db::model::{GetPolicy, GetProposal, GetUtxo};
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util::{self, format};
use iced::widget::{Column, Container, PickList, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::{Dashboard, FeeSelector, PolicyTree, UtxoSelector};
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, NumericInput, Text, TextInput};
use crate::theme::color::{DARK_RED, RED};

#[derive(Debug, Clone, Eq)]
pub struct PolicyPicLisk {
    pub policy_id: EventId,
    pub policy: Policy,
}

impl PartialEq for PolicyPicLisk {
    fn eq(&self, other: &Self) -> bool {
        self.policy_id == other.policy_id
    }
}

impl fmt::Display for PolicyPicLisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - #{}",
            self.policy.name,
            util::cut_event_id(self.policy_id)
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub enum InternalStageBuild {
    #[default]
    Details,
    Utxos,
}

#[derive(Debug, Clone, Copy)]
pub enum InternalStage {
    Build(InternalStageBuild),
    SelectPolicyPath,
    Review,
}

impl Default for InternalStage {
    fn default() -> Self {
        Self::Build(InternalStageBuild::Details)
    }
}

#[derive(Debug, Clone)]
pub enum SpendMessage {
    LoadPolicies(Vec<PolicyPicLisk>),
    PolicySelectd(PolicyPicLisk),
    LoadPolicy(EventId),
    AddressChanged(String),
    AmountChanged(Option<u64>),
    SendAllBtnPressed,
    DescriptionChanged(String),
    FeeRateChanged(FeeRate),
    PolicyLoaded(
        Option<Balance>,
        Vec<GetUtxo>,
        SatisfiableItem,
        Vec<(String, Vec<String>)>,
    ),
    SelectedUtxosChanged(HashSet<OutPoint>),
    ToggleCondition(String, usize),
    ErrorChanged(Option<String>),
    SetInternalStage(InternalStage),
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
    utxos: Vec<GetUtxo>,
    selected_utxos: HashSet<OutPoint>,
    policy_path: Option<BTreeMap<String, Vec<usize>>>,
    balance: Option<Balance>,
    satisfiable_item: Option<SatisfiableItem>,
    selectable_conditions: Option<Vec<(String, Vec<String>)>>,
    stage: InternalStage,
    loading: bool,
    loaded: bool,
    error: Option<String>,
}

impl SpendState {
    pub fn new(policy: Option<(EventId, Policy)>) -> Self {
        Self {
            policy: policy.map(|(policy_id, policy)| PolicyPicLisk { policy_id, policy }),
            policies: Vec::new(),
            to_address: String::new(),
            amount: None,
            send_all: false,
            description: String::new(),
            fee_rate: FeeRate::default(),
            utxos: Vec::new(),
            selected_utxos: HashSet::new(),
            policy_path: None,
            balance: None,
            satisfiable_item: None,
            selectable_conditions: None,
            stage: InternalStage::default(),
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
        let fee_rate = self.fee_rate;
        let selected_utxos: Vec<OutPoint> = self.selected_utxos.iter().cloned().collect();
        let policy_path = self.policy_path.clone();

        Command::perform(
            async move {
                let GetProposal { proposal_id, .. } = client
                    .spend(
                        policy_id,
                        to_address,
                        amount,
                        description,
                        fee_rate,
                        if selected_utxos.is_empty() {
                            None
                        } else {
                            Some(selected_utxos)
                        },
                        policy_path,
                    )
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
                         }| PolicyPicLisk { policy_id, policy },
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
                            SpendMessage::LoadPolicy(policy_id).into()
                        });
                    }
                }
                SpendMessage::PolicySelectd(policy) => {
                    let policy_id = policy.policy_id;
                    self.policy = Some(policy);
                    return Command::perform(async {}, move |_| {
                        SpendMessage::LoadPolicy(policy_id).into()
                    });
                }
                SpendMessage::LoadPolicy(policy_id) => {
                    let client = ctx.client.clone();
                    if let Some(policy) = self.policy.as_ref() {
                        let policy = policy.policy.clone();
                        return Command::perform(
                            async move {
                                let balance = client.get_balance(policy_id).await;
                                let utxos = client.get_utxos(policy_id).await?;
                                let item = policy.satisfiable_item(client.network())?;
                                let conditions = policy.selectable_conditions(client.network())?;
                                Ok::<
                                    (
                                        Option<Balance>,
                                        Vec<GetUtxo>,
                                        SatisfiableItem,
                                        Vec<(String, Vec<String>)>,
                                    ),
                                    Box<dyn std::error::Error>,
                                >((
                                    balance, utxos, item, conditions,
                                ))
                            },
                            |res| match res {
                                Ok((balance, utxos, item, conditions)) => {
                                    SpendMessage::PolicyLoaded(balance, utxos, item, conditions)
                                        .into()
                                }
                                Err(e) => SpendMessage::ErrorChanged(Some(format!(
                                    "Impossible to load policy: {e}",
                                )))
                                .into(),
                            },
                        );
                    } else {
                        self.error = Some(String::from("Select a policy"));
                    }
                }
                SpendMessage::PolicyLoaded(balance, utxos, item, conditions) => {
                    self.balance = balance;
                    self.utxos = utxos;
                    self.satisfiable_item = Some(item);
                    self.selectable_conditions = Some(conditions);
                }
                SpendMessage::SelectedUtxosChanged(s) => self.selected_utxos = s,
                SpendMessage::ToggleCondition(id, index) => match self.policy_path.as_mut() {
                    Some(policy_path) => match policy_path.get_mut(&id) {
                        Some(v) => {
                            if v.contains(&index) {
                                *v = v
                                    .iter()
                                    .filter(|i| **i != index)
                                    .copied()
                                    .collect::<Vec<usize>>();
                            } else {
                                v.push(index);
                            }
                        }
                        None => {
                            policy_path.insert(id, vec![index]);
                        }
                    },
                    None => {
                        let mut path = BTreeMap::new();
                        path.insert(id, vec![index]);
                        self.policy_path = Some(path);
                    }
                },
                SpendMessage::AddressChanged(value) => self.to_address = value,
                SpendMessage::AmountChanged(value) => self.amount = value,
                SpendMessage::SendAllBtnPressed => self.send_all = !self.send_all,
                SpendMessage::DescriptionChanged(value) => self.description = value,
                SpendMessage::FeeRateChanged(fee_rate) => self.fee_rate = fee_rate,
                SpendMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                SpendMessage::SetInternalStage(stage) => match stage {
                    InternalStage::Build(_) => self.stage = stage,
                    _ => match &self.policy {
                        Some(_) => match Address::from_str(&self.to_address) {
                            Ok(_) => {
                                if self.send_all {
                                    self.error = None;
                                    self.stage = stage;
                                } else {
                                    match self.amount {
                                        Some(_) => {
                                            self.error = None;
                                            self.stage = stage;
                                        }
                                        None => self.error = Some(String::from("Invalid amount")),
                                    };
                                }
                            }
                            Err(e) => self.error = Some(e.to_string()),
                        },
                        None => self.error = Some(String::from("You must select a policy")),
                    },
                },
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
            content = match self.stage {
                InternalStage::Build(stage) => self.view_build_tx(stage),
                InternalStage::SelectPolicyPath => self.view_policy_tree(),
                InternalStage::Review => self.view_review(),
            };
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

impl SpendState {
    fn view_build_tx<'a>(&self, stage: InternalStageBuild) -> Column<'a, Message> {
        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let (next_stage, ready): (InternalStage, bool) = {
            match &self.policy {
                Some(policy) => {
                    let descriptor = policy.policy.descriptor.to_string();
                    if descriptor.contains("after") || descriptor.contains("older") {
                        (InternalStage::SelectPolicyPath, true)
                    } else {
                        (InternalStage::Review, true)
                    }
                }
                None => (InternalStage::default(), false),
            }
        };

        let continue_btn = Button::new()
            .text("Continue")
            .width(Length::Fixed(400.0))
            .loading(
                !ready || self.to_address.is_empty() || (self.amount.is_none() && !self.send_all),
            )
            .on_press(SpendMessage::SetInternalStage(next_stage).into())
            .view();

        Column::new()
            .spacing(10)
            .padding(20)
            .push(
                Column::new()
                    .push(Text::new("Send").big().bold().view())
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
                    .push(
                        Button::new()
                            .style(if let InternalStageBuild::Details = stage {
                                ButtonStyle::Primary
                            } else {
                                ButtonStyle::Bordered
                            })
                            .text("Details")
                            .width(Length::Fill)
                            .on_press(
                                SpendMessage::SetInternalStage(InternalStage::Build(
                                    InternalStageBuild::Details,
                                ))
                                .into(),
                            )
                            .view(),
                    )
                    .push(
                        Button::new()
                            .style(if let InternalStageBuild::Utxos = stage {
                                ButtonStyle::Primary
                            } else {
                                ButtonStyle::Bordered
                            })
                            .text("UTXOs")
                            .width(Length::Fill)
                            .loading(self.loading || self.policy.is_none())
                            .on_press(
                                SpendMessage::SetInternalStage(InternalStage::Build(
                                    InternalStageBuild::Utxos,
                                ))
                                .into(),
                            )
                            .view(),
                    )
                    .spacing(5),
            )
            .push(Space::with_height(Length::Fixed(5.0)))
            .push(match stage {
                InternalStageBuild::Details => self.view_details(),
                InternalStageBuild::Utxos => self.view_utxos(),
            })
            .push(Space::with_height(Length::Fixed(5.0)))
            .push(error)
            .push(Space::with_height(Length::Fixed(5.0)))
            .push(continue_btn)
            .max_width(850.0)
    }

    fn view_details<'a>(&self) -> Column<'a, Message> {
        let policy_pick_list = Column::new()
            .push(Text::new("Policy").view())
            .push(
                PickList::new(self.policies.clone(), self.policy.clone(), |policy| {
                    SpendMessage::PolicySelectd(policy).into()
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
                    .small()
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
                    format!(
                        "Balance: {} sat",
                        format::number(balance.trusted_spendable())
                    )
                }
                None => String::from("Loading..."),
            })
            .extra_light()
            .small()
            .width(Length::Fill)
            .view()
        } else {
            Text::new("").view()
        };

        let description = TextInput::new("Description", &self.description)
            .on_input(|s| SpendMessage::DescriptionChanged(s).into())
            .placeholder("Description")
            .view();

        let details = Column::new()
            .push(policy_pick_list)
            .push(address)
            .push(amount)
            .push(your_balance)
            .push(description)
            .spacing(10)
            .max_width(400);

        Column::new().push(
            Row::new()
                .push(details)
                .push(rule::vertical())
                .push(
                    FeeSelector::new(self.fee_rate, |f| SpendMessage::FeeRateChanged(f).into())
                        .max_width(400.0),
                )
                .spacing(25)
                .height(Length::Fixed(375.0)),
        )
    }

    fn view_utxos<'a>(&self) -> Column<'a, Message> {
        Column::new().push(UtxoSelector::new(
            self.utxos.clone(),
            self.selected_utxos.clone(),
            |s| SpendMessage::SelectedUtxosChanged(s).into(),
        ))
    }

    fn view_policy_tree<'a>(&self) -> Column<'a, Message> {
        let tree = match self.satisfiable_item.clone() {
            Some(item) => PolicyTree::new(item).view(),
            None => Column::new().push(
                Text::new("Impossible to load policy tree")
                    .color(RED)
                    .view(),
            ),
        };

        let checkboxes = match self.selectable_conditions.clone() {
            Some(conditions) => {
                let policy_path = self.policy_path.clone().unwrap_or_default();
                let mut checkboxes = Column::new()
                    .spacing(5)
                    .padding(20)
                    .align_items(Alignment::Center);

                if !conditions.is_empty() {
                    checkboxes = checkboxes
                        .push(Text::new("Select conditions").view())
                        .push(Space::with_height(Length::Fixed(5.0)));

                    for (id, list) in conditions.into_iter() {
                        let pp_list = policy_path.get(&id);
                        for (index, sub_id) in list.into_iter().enumerate() {
                            let selected: bool = match pp_list {
                                Some(pp_list) => pp_list.contains(&index),
                                None => false,
                            };
                            checkboxes = checkboxes.push(
                                Button::new()
                                    .text(sub_id)
                                    .style(if selected {
                                        ButtonStyle::Primary
                                    } else {
                                        ButtonStyle::Bordered
                                    })
                                    .on_press(
                                        SpendMessage::ToggleCondition(id.clone(), index).into(),
                                    )
                                    .width(Length::Fixed(250.0))
                                    .view(),
                            );
                        }
                    }
                } else {
                    checkboxes = checkboxes.push(Text::new("No conditions to select").view());
                }

                checkboxes
            }
            None => Column::new().push(
                Text::new("Impossible to load selectable conditions")
                    .color(RED)
                    .view(),
            ),
        };

        let next = Button::new()
            .text("Next")
            .width(Length::Fill)
            .on_press(SpendMessage::SetInternalStage(InternalStage::Review).into())
            .loading(self.loading)
            .width(Length::Fixed(400.0))
            .view();
        let back_btn = Button::new()
            .style(ButtonStyle::Bordered)
            .text("Back")
            .width(Length::Fill)
            .on_press(SpendMessage::SetInternalStage(InternalStage::default()).into())
            .width(Length::Fixed(400.0))
            .loading(self.loading)
            .view();

        Column::new()
            .spacing(10)
            .padding(20)
            .push(tree)
            .push(Space::with_height(Length::Fixed(40.0)))
            .push(checkboxes)
            .push(next)
            .push(back_btn)
    }

    fn view_review<'a>(&self) -> Column<'a, Message> {
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
            .push(Row::new().push(Text::new(self.fee_rate.to_string()).view()))
            .spacing(5)
            .width(Length::Fill);

        let error = if let Some(error) = &self.error {
            Row::new().push(Text::new(error).color(DARK_RED).view())
        } else {
            Row::new()
        };

        let prev_stage: InternalStage = {
            match &self.policy {
                Some(policy) => {
                    let descriptor = policy.policy.descriptor.to_string();
                    if descriptor.contains("after") || descriptor.contains("older") {
                        InternalStage::SelectPolicyPath
                    } else {
                        InternalStage::default()
                    }
                }
                None => InternalStage::default(),
            }
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
            .on_press(SpendMessage::SetInternalStage(prev_stage).into())
            .loading(self.loading)
            .view();

        Column::new()
            .spacing(10)
            .padding(20)
            .push(policy)
            .push(address)
            .push(amount)
            .push(description)
            .push(priority)
            .push(error)
            .push(Space::with_height(Length::Fixed(15.0)))
            .push(send_proposal_btn)
            .push(back_btn)
            .max_width(400)
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
