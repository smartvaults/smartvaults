// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashSet;

use iced::widget::{component, Column, Component, Row, Space};
use iced::{Alignment, Element, Length, Renderer};
use smartvaults_sdk::core::bdk::chain::ConfirmationTime;
use smartvaults_sdk::core::bdk::LocalUtxo;
use smartvaults_sdk::core::bitcoin::OutPoint;
use smartvaults_sdk::types::GetUtxo;
use smartvaults_sdk::util::format;

use crate::app::Message;
use crate::component::{rule, Button, ButtonStyle, Text};

#[derive(Debug, Clone)]
pub enum Event {
    ToggleUtxo(OutPoint),
}

pub struct UtxoSelector {
    utxos: Vec<GetUtxo>,
    selected_utxos: HashSet<OutPoint>,
    on_select: Box<dyn Fn(HashSet<OutPoint>) -> Message>,
}

impl UtxoSelector {
    pub fn new(
        utxos: Vec<GetUtxo>,
        selected_utxos: HashSet<OutPoint>,
        on_select: impl Fn(HashSet<OutPoint>) -> Message + 'static,
    ) -> Self {
        Self {
            utxos,
            selected_utxos,
            on_select: Box::new(on_select),
        }
    }
}

impl Component<Message, Renderer> for UtxoSelector {
    type State = ();
    type Event = Event;

    fn update(&mut self, _state: &mut Self::State, event: Event) -> Option<Message> {
        match event {
            Event::ToggleUtxo(utxo) => {
                if self.selected_utxos.contains(&utxo) {
                    self.selected_utxos.remove(&utxo);
                } else {
                    self.selected_utxos.insert(utxo);
                }

                Some((self.on_select)(self.selected_utxos.clone()))
            }
        }
    }

    fn view(&self, _state: &Self::State) -> Element<Event, Renderer> {
        let mut content = Column::new()
            .spacing(10)
            .push(
                Row::new()
                    .push(
                        Text::new("UTXO")
                            .bold()
                            .big()
                            .width(Length::Fixed(180.0))
                            .view(),
                    )
                    .push(Text::new("Value").bold().big().width(Length::Fill).view())
                    .push(Text::new("Label").bold().big().width(Length::Fill).view())
                    .push(
                        Text::new("Block Height")
                            .bold()
                            .big()
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(Space::with_width(Length::Fixed(130.0)))
                    .spacing(20)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(rule::horizontal_bold());

        for GetUtxo {
            utxo,
            label,
            frozen,
        } in self.utxos.iter()
        {
            let LocalUtxo {
                outpoint,
                txout,
                confirmation_time,
                ..
            } = utxo;
            let selected: bool = self.selected_utxos.contains(outpoint);
            let txid: String = outpoint.txid.to_string();
            content = content
                .push(
                    Row::new()
                        .push(
                            Text::new(format!(
                                "{}..{}:{}",
                                &txid[..8],
                                &txid[txid.len() - 8..],
                                outpoint.vout,
                            ))
                            .width(Length::Fixed(180.0))
                            .view(),
                        )
                        .push(
                            Text::new(format!("{} sat", format::number(txout.value)))
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(label.clone().unwrap_or_else(|| String::from("-")))
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(match confirmation_time {
                                ConfirmationTime::Confirmed { height, .. } => {
                                    format::number(*height as u64)
                                }
                                ConfirmationTime::Unconfirmed { .. } => String::from("Pending"),
                            })
                            .width(Length::Fill)
                            .view(),
                        )
                        .push(if *frozen {
                            Button::new()
                                .text("Frozen")
                                .style(ButtonStyle::Bordered)
                                .width(Length::Fixed(130.0))
                                .view()
                        } else {
                            Button::new()
                                .text(if selected { "Selected" } else { "Select" })
                                .style(if selected {
                                    ButtonStyle::Primary
                                } else {
                                    ButtonStyle::Bordered
                                })
                                .on_press(Event::ToggleUtxo(*outpoint))
                                .width(Length::Fixed(130.0))
                                .view()
                        })
                        .spacing(20)
                        .align_items(Alignment::Center),
                )
                .push(rule::horizontal());
        }

        content.into()
    }
}

impl<'a> From<UtxoSelector> for Element<'a, Message, Renderer> {
    fn from(numeric_input: UtxoSelector) -> Self {
        component(numeric_input)
    }
}
