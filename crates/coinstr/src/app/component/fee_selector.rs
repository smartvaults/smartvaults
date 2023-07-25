// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::FeeRate;
use iced::{
    widget::{Column, Radio, Row},
    Element, Length, Renderer,
};
use iced_lazy::Component;
use iced_native::Alignment;

use crate::app::Message;
use crate::component::Text;

#[derive(Debug, Clone)]
pub enum Event {
    FeeRateChanged(FeeRate),
}

pub struct FeeSelector {
    fee_rate: FeeRate,
    on_change: Box<dyn Fn(FeeRate) -> Message>,
}

impl FeeSelector {
    pub fn new(fee_rate: FeeRate, on_change: impl Fn(FeeRate) -> Message + 'static) -> Self {
        Self {
            fee_rate,
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
        }
    }

    fn view(&self, _state: &Self::State) -> Element<Event, Renderer> {
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

        /* let custom_priority = Row::new()
        .push(Radio::new(
            "",
            FeeRate::Custom(self.custom_target_blocks.unwrap_or_default() as usize),
            Some(self.fee_rate),
            |fee_rate| SpendMessage::FeeRateChanged(fee_rate).into(),
        ))
        .push(
            NumericInput::new("", self.custom_target_blocks)
                .placeholder("Target blocks")
                .on_input(|b| SpendMessage::CustomTargetBlockChanged(b).into()),
        )
        .align_items(Alignment::Center)
        .width(Length::Fill); */

        Column::new()
            .push(Text::new("Priority & arrival time").view())
            .push(
                Column::new()
                    .push(fee_high_priority)
                    .push(fee_medium_priority)
                    .push(fee_low_priority)
                    //.push(custom_priority)
                    .spacing(10),
            )
            .spacing(5)
            .into()
    }
}

impl<'a> From<FeeSelector> for Element<'a, Message, Renderer> {
    fn from(numeric_input: FeeSelector) -> Self {
        iced_lazy::component(numeric_input)
    }
}
