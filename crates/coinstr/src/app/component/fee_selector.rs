// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::FeeRate;
use iced::{
    widget::{Column, Radio, Row},
    Element, Length, Renderer,
};
use iced_lazy::Component;
use iced_native::Alignment;

use crate::component::{NumericInput, Text};
use crate::{
    app::Message,
    component::{Button, ButtonStyle},
};

#[derive(Debug, Clone, Copy, Default)]
pub enum InternalStage {
    #[default]
    TargetBlocks,
    FeeRate,
}

#[derive(Debug, Clone)]
pub enum Event {
    FeeRateChanged(FeeRate),
    CustomTargetBlockChanged(Option<u64>),
    SetInternalStage(InternalStage),
}

pub struct FeeSelector {
    fee_rate: FeeRate,
    custom_target_blocks: Option<u64>,
    stage: InternalStage,
    on_change: Box<dyn Fn(FeeRate) -> Message>,
}

impl FeeSelector {
    pub fn new(fee_rate: FeeRate, on_change: impl Fn(FeeRate) -> Message + 'static) -> Self {
        Self {
            fee_rate,
            custom_target_blocks: if let FeeRate::Custom(target) = fee_rate {
                Some(target as u64)
            } else {
                None
            },
            stage: InternalStage::default(),
            on_change: Box::new(on_change),
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
                Some(target) => Some((self.on_change)(FeeRate::Custom(target as usize))),
                None => Some((self.on_change)(FeeRate::default())),
            },
            Event::SetInternalStage(stage) => {
                self.stage = stage;
                None
            }
        }
    }

    fn view(&self, _state: &Self::State) -> Element<Event, Renderer> {
        Column::new()
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
                            //.on_press(Event::SetInternalStage(InternalStage::FeeRate))
                            .view(),
                    )
                    .spacing(5),
            )
            .push(match self.stage {
                InternalStage::TargetBlocks => self.view_target_blocks(),
                InternalStage::FeeRate => self.view_fee_rate(),
            })
            .spacing(10)
            .into()
    }
}

impl FeeSelector {
    fn view_target_blocks<'a>(&self) -> Column<'a, Event> {
        let fee_high_priority = Row::new()
            .push(Radio::new(
                "",
                FeeRate::High,
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
                FeeRate::Medium,
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
                FeeRate::Low,
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
                FeeRate::Custom(self.custom_target_blocks.unwrap_or_default() as usize),
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
                NumericInput::new("Fee rate (sat/vByte)", self.custom_target_blocks)
                    .placeholder("sat/vByte"),
            )
            .spacing(10)
    }
}

impl<'a> From<FeeSelector> for Element<'a, Message, Renderer> {
    fn from(numeric_input: FeeSelector) -> Self {
        iced_lazy::component(numeric_input)
    }
}
