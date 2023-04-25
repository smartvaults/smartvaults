// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_core::nostr_sdk::EventId;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::util::{self, format};
use iced::widget::{Column, Row};
use iced::{Alignment, Length};

use crate::app::{Message, Stage};
use crate::component::{button, rule, Text};
use crate::theme::icon::FULLSCREEN;

pub struct SpendingProposalsList {
    map: BTreeMap<EventId, (EventId, SpendingProposal)>,
    take: Option<usize>,
}

impl SpendingProposalsList {
    pub fn new(map: BTreeMap<EventId, (EventId, SpendingProposal)>) -> Self {
        Self { map, take: None }
    }

    pub fn take(self, num: usize) -> Self {
        Self {
            take: Some(num),
            ..self
        }
    }

    pub fn view(self) -> Column<'static, Message> {
        let mut proposals = Column::new()
            .push(
                Row::new()
                    .push(
                        Text::new("ID")
                            .bold()
                            .bigger()
                            .width(Length::Fixed(115.0))
                            .view(),
                    )
                    .push(
                        Text::new("Policy ID")
                            .bold()
                            .bigger()
                            .width(Length::Fixed(115.0))
                            .view(),
                    )
                    .push(
                        Text::new("Amount")
                            .bold()
                            .bigger()
                            .width(Length::Fixed(125.0))
                            .view(),
                    )
                    .push(
                        Text::new("Description")
                            .bold()
                            .bigger()
                            .width(Length::Fill)
                            .view(),
                    )
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(rule::horizontal_bold())
            .width(Length::Fill)
            .spacing(10);

        if self.map.is_empty() {
            proposals = proposals.push(Text::new("No proposals").extra_light().view());
        } else {
            for (proposal_id, (policy_id, proposal)) in self.map.iter() {
                let row = Row::new()
                    .push(
                        Text::new(util::cut_event_id(*proposal_id))
                            .width(Length::Fixed(115.0))
                            .view(),
                    )
                    .push(
                        Text::new(util::cut_event_id(*policy_id))
                            .width(Length::Fixed(115.0))
                            .on_press(Message::View(Stage::Policy(*policy_id)))
                            .view(),
                    )
                    .push(
                        Text::new(format!("{} sat", format::big_number(proposal.amount)))
                            .width(Length::Fixed(125.0))
                            .view(),
                    )
                    .push(Text::new(&proposal.description).width(Length::Fill).view())
                    .push(
                        button::primary_only_icon(FULLSCREEN)
                            .on_press(Message::View(Stage::Proposal(
                                *proposal_id,
                                proposal.clone(),
                            )))
                            .width(Length::Fixed(40.0)),
                    )
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill);
                proposals = proposals.push(row).push(rule::horizontal());
            }
        }

        if let Some(take) = self.take {
            if self.map.len() > take {
                proposals = proposals.push(
                    Text::new("Show all")
                        .on_press(Message::View(Stage::Proposals))
                        .view(),
                );
            }
        }

        proposals
    }
}
