// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashMap;
use std::fmt;
use std::ops::Deref;
use std::str::FromStr;

use iced::widget::{Column, PickList, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::nostr::EventId;
use smartvaults_sdk::protocol::v1::key_agent::signer::Currency;
use smartvaults_sdk::protocol::v1::{BasisPoints, DeviceType, Price, SignerOffering, Temperature};
use smartvaults_sdk::types::GetSigner;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{Button, ButtonStyle, NumericInput, Text, TextInput};
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone, Eq)]
pub struct SignerPickLisk {
    signer: GetSigner,
    offering: Option<SignerOffering>,
}

impl PartialEq for SignerPickLisk {
    fn eq(&self, other: &Self) -> bool {
        self.signer.signer_id == other.signer.signer_id
    }
}

impl fmt::Display for SignerPickLisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - {}",
            self.signer.signer.name(),
            self.signer.signer.fingerprint(),
        )
    }
}

impl Deref for SignerPickLisk {
    type Target = GetSigner;
    fn deref(&self) -> &Self::Target {
        &self.signer
    }
}

impl From<(GetSigner, Option<SignerOffering>)> for SignerPickLisk {
    fn from(value: (GetSigner, Option<SignerOffering>)) -> Self {
        Self {
            signer: value.0,
            offering: value.1,
        }
    }
}

#[derive(Debug, Clone)]
pub enum EditSignerOfferingMessage {
    Load(Vec<SignerPickLisk>),
    SignerSelectd(SignerPickLisk),
    TemperatureChanged(Temperature),
    DeviceTypeChanged(DeviceType),
    ResponseTimeChanged(Option<u16>),
    YearlyCostBasisPointsChanged(Option<u64>),
    YearlyCostChanged(Option<u64>),
    YearlyCostCurrencyChanged(String),
    CostPerSignatureChanged(Option<u64>),
    CostPerSignatureCurrencyChanged(String),
    Save,
    ErrorChanged(Option<String>),
    Reload,
}

#[derive(Debug)]
pub struct EditSignerOfferingState {
    signer: Option<SignerPickLisk>,
    signers: Vec<SignerPickLisk>,
    temperature: Option<Temperature>,
    response_time: Option<u16>,
    device_type: Option<DeviceType>,
    yearly_cost_basis_points: Option<u64>,
    cost_per_signature: Option<u64>,
    cost_per_signature_currency: String,
    yearly_cost: Option<u64>,
    yearly_cost_currency: String,
    loading: bool,
    loaded: bool,
    allow_reload: bool,
    error: Option<String>,
}

impl EditSignerOfferingState {
    pub fn new(signer: Option<(GetSigner, Option<SignerOffering>)>) -> Self {
        Self {
            signer: signer.map(|p| p.into()),
            signers: Vec::new(),
            temperature: None,
            response_time: None,
            device_type: None,
            yearly_cost_basis_points: None,
            cost_per_signature: None,
            cost_per_signature_currency: String::new(),
            yearly_cost: None,
            yearly_cost_currency: String::new(),
            loading: false,
            loaded: false,
            allow_reload: false,
            error: None,
        }
    }
}

impl State for EditSignerOfferingState {
    fn title(&self) -> String {
        String::from("Edit signer offering")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loaded && !self.allow_reload {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let offerings: HashMap<EventId, SignerOffering> = client
                    .my_signer_offerings()
                    .await
                    .unwrap()
                    .into_iter()
                    .map(|s| (s.signer.signer_id, s.offering))
                    .collect();
                client
                    .get_signers()
                    .await
                    .into_iter()
                    .map(|p| {
                        let offering: Option<SignerOffering> = offerings.get(&p.signer_id).cloned();
                        (p, offering).into()
                    })
                    .collect()
            },
            |p| EditSignerOfferingMessage::Load(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::EditSignerOffering(msg) = message {
            match msg {
                EditSignerOfferingMessage::Load(signers) => {
                    self.signers = signers;
                    self.loading = false;
                    self.loaded = true;
                    self.allow_reload = false;
                    if let Some(signer) = self.signer.clone() {
                        return Command::perform(async move {}, |_| {
                            EditSignerOfferingMessage::SignerSelectd(signer).into()
                        });
                    }
                }
                EditSignerOfferingMessage::SignerSelectd(signer) => {
                    match &signer.offering {
                        Some(offering) => {
                            self.temperature = Some(offering.temperature);
                            self.response_time = Some(offering.response_time);
                            self.device_type = Some(offering.device_type);
                            self.yearly_cost_basis_points =
                                offering.yearly_cost_basis_points.map(|p| *p);
                            self.cost_per_signature =
                                offering.cost_per_signature.as_ref().map(|p| p.amount);
                            self.cost_per_signature_currency = offering
                                .cost_per_signature
                                .as_ref()
                                .map(|p| p.currency.to_string())
                                .unwrap_or_default();
                            self.yearly_cost = offering.yearly_cost.as_ref().map(|p| p.amount);
                            self.yearly_cost_currency = offering
                                .yearly_cost
                                .as_ref()
                                .map(|p| p.currency.to_string())
                                .unwrap_or_default();
                        }
                        None => {
                            self.temperature = None;
                            self.response_time = None;
                            self.device_type = None;
                            self.yearly_cost_basis_points = None;
                            self.cost_per_signature = None;
                            self.cost_per_signature_currency.clear();
                            self.yearly_cost = None;
                            self.yearly_cost_currency.clear();
                        }
                    }
                    self.signer = Some(signer);
                }
                EditSignerOfferingMessage::TemperatureChanged(temperature) => {
                    self.temperature = Some(temperature);
                }
                EditSignerOfferingMessage::DeviceTypeChanged(device_type) => {
                    self.device_type = Some(device_type);
                }
                EditSignerOfferingMessage::ResponseTimeChanged(response_time) => {
                    self.response_time = response_time;
                }
                EditSignerOfferingMessage::YearlyCostBasisPointsChanged(
                    yearly_cost_basis_points,
                ) => {
                    self.yearly_cost_basis_points = yearly_cost_basis_points;
                }
                EditSignerOfferingMessage::YearlyCostChanged(yearly_cost) => {
                    self.yearly_cost = yearly_cost;
                }
                EditSignerOfferingMessage::YearlyCostCurrencyChanged(yearly_cost_currency) => {
                    self.yearly_cost_currency = yearly_cost_currency;
                }
                EditSignerOfferingMessage::CostPerSignatureChanged(cost_per_signature) => {
                    self.cost_per_signature = cost_per_signature;
                }
                EditSignerOfferingMessage::CostPerSignatureCurrencyChanged(
                    cost_per_signature_currency,
                ) => {
                    self.cost_per_signature_currency = cost_per_signature_currency;
                }
                EditSignerOfferingMessage::Save => {
                    let client = ctx.client.clone();
                    if let Some(signer) = self.signer.as_ref() {
                        if let Some(temperature) = &self.temperature {
                            if let Some(device_type) = &self.device_type {
                                // Cost per signature
                                let cost_per_signature: Option<Price> =
                                    match self.cost_per_signature {
                                        Some(amount) => match Currency::from_str(
                                            &self.cost_per_signature_currency,
                                        ) {
                                            Ok(currency) => Some(Price { amount, currency }),
                                            Err(e) => {
                                                self.error = Some(e.to_string());
                                                return Command::none();
                                            }
                                        },
                                        None => None,
                                    };

                                // Yearly cost
                                let yearly_cost: Option<Price> = match self.yearly_cost {
                                    Some(amount) => {
                                        match Currency::from_str(&self.yearly_cost_currency) {
                                            Ok(currency) => Some(Price { amount, currency }),
                                            Err(e) => {
                                                self.error = Some(e.to_string());
                                                return Command::none();
                                            }
                                        }
                                    }
                                    None => None,
                                };

                                self.loading = true;
                                let signer = signer.signer.deref().clone();
                                let offering = SignerOffering {
                                    temperature: *temperature,
                                    response_time: self.response_time.unwrap_or(3600),
                                    device_type: *device_type,
                                    cost_per_signature,
                                    yearly_cost,
                                    yearly_cost_basis_points: self
                                        .yearly_cost_basis_points
                                        .map(BasisPoints::from),
                                    network: client.network(),
                                };
                                return Command::perform(
                                    async move { client.signer_offering(&signer, offering).await },
                                    |res| match res {
                                        Ok(_) => Message::View(Stage::Signers),
                                        Err(e) => EditSignerOfferingMessage::ErrorChanged(Some(
                                            e.to_string(),
                                        ))
                                        .into(),
                                    },
                                );
                            } else {
                                self.error = Some(String::from("Device type not selected"));
                            }
                        } else {
                            self.error = Some(String::from("Temperature not selected"));
                        }
                    } else {
                        self.error = Some(String::from("Signer not selected"));
                    }
                }
                EditSignerOfferingMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                EditSignerOfferingMessage::Reload => {
                    self.allow_reload = true;
                    return self.load(ctx);
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new();

        if self.loaded {
            content = content
                .push(
                    Column::new()
                        .push(Text::new("Signer offering").big().bold().view())
                        .push(
                            Text::new("Create/Edit signer offering")
                                .extra_light()
                                .view(),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(
                    Column::new()
                        .push(Text::new("Signer").view())
                        .push(
                            PickList::new(self.signers.clone(), self.signer.clone(), |signer| {
                                EditSignerOfferingMessage::SignerSelectd(signer).into()
                            })
                            .width(Length::Fill)
                            .padding(10)
                            .placeholder(if self.signers.is_empty() {
                                "No signer availabe"
                            } else {
                                "Select a signer"
                            }),
                        )
                        .spacing(5),
                );

            if self.signer.is_some() {
                let temperature = Column::new()
                    .push(Text::new("Temperature").view())
                    .push(
                        PickList::new(Temperature::list(), self.temperature, |temperature| {
                            EditSignerOfferingMessage::TemperatureChanged(temperature).into()
                        })
                        .width(Length::Fill)
                        .padding(10)
                        .placeholder("Select temperature"),
                    )
                    .spacing(5);

                let device_type = Column::new()
                    .push(Text::new("Device type").view())
                    .push(
                        PickList::new(DeviceType::list(), self.device_type, |device_type| {
                            EditSignerOfferingMessage::DeviceTypeChanged(device_type).into()
                        })
                        .width(Length::Fill)
                        .padding(10)
                        .placeholder("Select device type"),
                    )
                    .spacing(5);

                let response_time = NumericInput::new("Response time (min)", self.response_time)
                    .placeholder("Response time (min)")
                    .on_input(|r| EditSignerOfferingMessage::ResponseTimeChanged(r).into());

                let yearly_cost_basis_points = NumericInput::new(
                    "Yearly cost (basis points - optional)",
                    self.yearly_cost_basis_points,
                )
                .placeholder("Yearly cost (basis points - optional)")
                .on_input(|r| EditSignerOfferingMessage::YearlyCostBasisPointsChanged(r).into());

                let yearly_cost = Row::new()
                    .spacing(5)
                    .push(
                        NumericInput::new("Yearly cost (optional)", self.yearly_cost)
                            .placeholder("Yearly cost (optional)")
                            .on_input(|r| EditSignerOfferingMessage::YearlyCostChanged(r).into())
                            .width(Length::FillPortion(3)),
                    )
                    .push(
                        TextInput::new(&self.yearly_cost_currency)
                            .label("Currency")
                            .placeholder("Currency")
                            .on_input(|s| {
                                EditSignerOfferingMessage::YearlyCostCurrencyChanged(s).into()
                            })
                            .view()
                            .width(Length::Fill),
                    )
                    .width(Length::Fill);

                let cost_per_signature = Row::new()
                    .spacing(5)
                    .push(
                        NumericInput::new("Cost per signature (optional)", self.cost_per_signature)
                            .placeholder("Cost per signature (optional)")
                            .on_input(|r| {
                                EditSignerOfferingMessage::CostPerSignatureChanged(r).into()
                            })
                            .width(Length::FillPortion(3)),
                    )
                    .push(
                        TextInput::new(&self.cost_per_signature_currency)
                            .label("Currency")
                            .placeholder("Currency")
                            .on_input(|s| {
                                EditSignerOfferingMessage::CostPerSignatureCurrencyChanged(s).into()
                            })
                            .view()
                            .width(Length::Fill),
                    )
                    .width(Length::Fill);

                let save_btn = Button::new()
                    .style(ButtonStyle::Primary)
                    .text("Save")
                    .loading(self.loading)
                    .on_press(EditSignerOfferingMessage::Save.into())
                    .width(Length::Fill)
                    .view();

                let error = if let Some(error) = &self.error {
                    Row::new().push(Text::new(error).color(DARK_RED).view())
                } else {
                    Row::new()
                };

                content = content
                    .push(temperature)
                    .push(device_type)
                    .push(response_time)
                    .push(yearly_cost_basis_points)
                    .push(yearly_cost)
                    .push(cost_per_signature)
                    .push(error)
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(save_btn);
            }

            content = content
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400)
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, true)
    }
}

impl From<EditSignerOfferingState> for Box<dyn State> {
    fn from(s: EditSignerOfferingState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<EditSignerOfferingMessage> for Message {
    fn from(msg: EditSignerOfferingMessage) -> Self {
        Self::EditSignerOffering(msg)
    }
}
