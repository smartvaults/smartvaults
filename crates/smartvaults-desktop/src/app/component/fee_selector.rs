// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{component, Column, Component, Radio, Row};
use iced::{Alignment, Element, Length, Renderer};
use smartvaults_sdk::core::{FeeRate, Priority};

use crate::app::Message;
use crate::component::{Button, ButtonStyle, NumericInput, Text};

#[derive(Debug, Clone, Copy, Default)]
pub enum InternalStage {
    #[default]
    TargetBlocks,
    FeeRate,
}

#[derive(Debug, Clone)]
pub enum Event {
    FeeRateChanged(FeeRate),
    CustomTargetBlockChanged(Option<u8>),
    CustomRateChanged(Option<f32>),
    SetInternalStage(InternalStage),
}

pub struct FeeSelector {
    fee_rate: FeeRate,
    custom_target_blocks: Option<u8>,
    custom_rate: Option<f32>,
    stage: InternalStage,
    max_width: Option<f32>,
    on_change: Box<dyn Fn(FeeRate) -> Message>,
}

impl FeeSelector {
    pub fn new(fee_rate: FeeRate, on_change: impl Fn(FeeRate) -> Message + 'static) -> Self {
        Self {
            fee_rate,
            custom_target_blocks: if let FeeRate::Priority(Priority::Custom(target)) = fee_rate {
                Some(target)
            } else {
                None
            },
            custom_rate: if let FeeRate::Rate(rate) = fee_rate {
                Some(rate)
            } else {
                None
            },
            stage: match fee_rate {
                FeeRate::Priority(..) => InternalStage::TargetBlocks,
                FeeRate::Rate(..) => InternalStage::FeeRate,
            },
            max_width: None,
            on_change: Box::new(on_change),
        }
    }

    pub fn max_width(self, width: f32) -> Self {
        Self {
            max_width: Some(width),
            ..self
        }
    }
}

impl Component<Message, Renderer> for FeeSelector {
    type State = ();
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Event) -> Option<Message> {
        match event {
            Event::FeeRateChanged(fee_rate) => Some((self.on_change)(fee_rate)),
            Event::CustomTargetBlockChanged(target) => match target {
                Some(target) => Some((self.on_change)(FeeRate::Priority(Priority::Custom(
                    target,
                )))),
                None => Some((self.on_change)(FeeRate::default())),
            },
            Event::CustomRateChanged(rate) => match rate {
                Some(rate) => Some((self.on_change)(FeeRate::Rate(rate))),
                None => Some((self.on_change)(FeeRate::min_relay_fee())),
            },
            Event::SetInternalStage(stage) => {
                self.stage = stage;
                match stage {
                    InternalStage::TargetBlocks => Some((self.on_change)(FeeRate::default())),
                    InternalStage::FeeRate => Some((self.on_change)(FeeRate::min_relay_fee())),
                }
            }
        }
    }

    fn view(&self, _state: &Self::State) -> Element<Event, Renderer> {
        let mut content = Column::new()
            .push(Text::new("Priority & arrival time").view())
            .push(
                Row::new()
                    .push(
                        Button::new()
                            .style(if let InternalStage::TargetBlocks = self.stage {
                                ButtonStyle::Primary
                            } else {
                                ButtonStyle::Bordered
                            })
                            .text("Target blocks")
                            .width(Length::Fill)
                            .on_press(Event::SetInternalStage(InternalStage::TargetBlocks))
                            .view(),
                    )
                    .push(
                        Button::new()
                            .style(if let InternalStage::FeeRate = self.stage {
                                ButtonStyle::Primary
                            } else {
                                ButtonStyle::Bordered
                            })
                            .text("Fee rate")
                            .width(Length::Fill)
                            .on_press(Event::SetInternalStage(InternalStage::FeeRate))
                            .view(),
                    )
                    .spacing(5),
            )
            .push(match self.stage {
                InternalStage::TargetBlocks => self.view_target_blocks(),
                InternalStage::FeeRate => self.view_fee_rate(),
            })
            .spacing(10);

        if let Some(max_width) = self.max_width {
            content = content.max_width(max_width);
        }

        content.into()
    }
}

impl FeeSelector {
    fn view_target_blocks<'a>(&self) -> Column<'a, Event> {
        let fee_high_priority = Row::new()
            .push(Radio::new(
                "",
                FeeRate::Priority(Priority::High),
                Some(self.fee_rate),
                Event::FeeRateChanged,
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
                FeeRate::Priority(Priority::Medium),
                Some(self.fee_rate),
                Event::FeeRateChanged,
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
                FeeRate::Priority(Priority::Low),
                Some(self.fee_rate),
                Event::FeeRateChanged,
            ))
            .push(
                Column::new()
                    .push(Text::new("Low").view())
                    .push(Text::new("1 - 2 hours").extra_light().size(18).view())
                    .spacing(5),
            )
            .align_items(Alignment::Center)
            .width(Length::Fill);

        let custom_priority = Row::new()
            .push(Radio::new(
                "",
                FeeRate::Priority(Priority::Custom(
                    self.custom_target_blocks.unwrap_or_default(),
                )),
                Some(self.fee_rate),
                Event::FeeRateChanged,
            ))
            .push(
                NumericInput::new("", self.custom_target_blocks)
                    .placeholder("Target blocks")
                    .on_input(Event::CustomTargetBlockChanged),
            )
            .align_items(Alignment::Center)
            .width(Length::Fill);

        Column::new()
            .push(fee_high_priority)
            .push(fee_medium_priority)
            .push(fee_low_priority)
            .push(custom_priority)
            .spacing(10)
    }

    fn view_fee_rate<'a>(&self) -> Column<'a, Event> {
        Column::new()
            .push(
                NumericInput::new("Fee rate (sat/vByte)", self.custom_rate)
                    .placeholder("sat/vByte")
                    .on_input(Event::CustomRateChanged),
            )
            .spacing(10)
    }
}

impl<'a> From<FeeSelector> for Element<'a, Message, Renderer> {
    fn from(numeric_input: FeeSelector) -> Self {
        component(numeric_input)
    }
}
