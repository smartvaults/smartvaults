// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::proposal::{CompletedProposal, Proposal};
use coinstr_sdk::types::{GetCompletedProposal, GetProposal};
use coinstr_sdk::util::{self, format};
use iced::widget::{Column, Row};
use iced::{Alignment, Length};

use crate::app::{Message, Stage};
use crate::component::{rule, Button, Text};
use crate::theme::icon::FULLSCREEN;

pub struct PendingProposalsList {
    map: Vec<GetProposal>,
    take: Option<usize>,
    hide_policy_id: bool,
}

impl PendingProposalsList {
    pub fn new(map: Vec<GetProposal>) -> Self {
        Self {
            map,
            take: None,
            hide_policy_id: false,
        }
    }

    pub fn take(self, num: usize) -> Self {
        Self {
            take: Some(num),
            ..self
        }
    }

    pub fn hide_policy_id(self) -> Self {
        Self {
            hide_policy_id: true,
            ..self
        }
    }

    pub fn view(self) -> Column<'static, Message> {
        let mut proposals = Column::new()
            .push(
                Row::new()
                    .push(Text::new("ID").bold().width(Length::Fixed(115.0)).view())
                    .push(if self.hide_policy_id {
                        Text::new("").view()
                    } else {
                        Text::new("Policy ID")
                            .bold()
                            .width(Length::Fixed(115.0))
                            .view()
                    })
                    .push(Text::new("Type").bold().width(Length::Fixed(125.0)).view())
                    .push(
                        Text::new("Amount")
                            .bold()
                            .width(Length::Fixed(125.0))
                            .view(),
                    )
                    .push(Text::new("Description").bold().width(Length::Fill).view())
                    .push(Text::new("Signed").bold().width(Length::Fill).view())
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
            for GetProposal {
                proposal_id,
                policy_id,
                proposal,
                signed,
            } in self.map.iter()
            {
                let row = match proposal {
                    Proposal::Spending {
                        amount,
                        description,
                        ..
                    } => Row::new()
                        .push(
                            Text::new(util::cut_event_id(*proposal_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Proposal(*proposal_id)))
                                .view(),
                        )
                        .push(if self.hide_policy_id {
                            Text::new("").view()
                        } else {
                            Text::new(util::cut_event_id(*policy_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Policy(*policy_id)))
                                .view()
                        })
                        .push(Text::new("spending").width(Length::Fixed(125.0)).view())
                        .push(
                            Text::new(format!("{} sat", format::big_number(*amount)))
                                .width(Length::Fixed(125.0))
                                .view(),
                        )
                        .push(Text::new(description).width(Length::Fill).view())
                        .push(Text::new(signed.to_string()).width(Length::Fill).view())
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::Proposal(*proposal_id)))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                    Proposal::ProofOfReserve { message, .. } => Row::new()
                        .push(
                            Text::new(util::cut_event_id(*proposal_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Proposal(*proposal_id)))
                                .view(),
                        )
                        .push(if self.hide_policy_id {
                            Text::new("").view()
                        } else {
                            Text::new(util::cut_event_id(*policy_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Policy(*policy_id)))
                                .view()
                        })
                        .push(
                            Text::new("proof-of-reserve")
                                .width(Length::Fixed(125.0))
                                .view(),
                        )
                        .push(Text::new("-").width(Length::Fixed(125.0)).view())
                        .push(Text::new(message).width(Length::Fill).view())
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::Proposal(*proposal_id)))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                };
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

pub struct CompletedProposalsList {
    map: Vec<GetCompletedProposal>,
    take: Option<usize>,
}

impl CompletedProposalsList {
    pub fn new(map: Vec<GetCompletedProposal>) -> Self {
        Self { map, take: None }
    }

    #[allow(dead_code)]
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
                    .push(Text::new("ID").bold().width(Length::Fixed(115.0)).view())
                    .push(
                        Text::new("Policy ID")
                            .bold()
                            .width(Length::Fixed(115.0))
                            .view(),
                    )
                    .push(Text::new("Type").bold().width(Length::Fixed(125.0)).view())
                    .push(Text::new("Description").bold().width(Length::Fill).view())
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
            for GetCompletedProposal {
                policy_id,
                completed_proposal_id,
                proposal,
            } in self.map.iter()
            {
                let row = match proposal {
                    CompletedProposal::Spending { description, .. } => Row::new()
                        .push(
                            Text::new(util::cut_event_id(*completed_proposal_id))
                                .width(Length::Fixed(115.0))
                                .view(),
                        )
                        .push(
                            Text::new(util::cut_event_id(*policy_id))
                                .width(Length::Fixed(115.0))
                                .on_press(Message::View(Stage::Policy(*policy_id)))
                                .view(),
                        )
                        .push(Text::new("spending").width(Length::Fixed(125.0)).view())
                        .push(Text::new(description).width(Length::Fill).view())
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::CompletedProposal(
                                    *completed_proposal_id,
                                )))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                    CompletedProposal::ProofOfReserve { message, .. } => Row::new()
                        .push(
                            Text::new(util::cut_event_id(*completed_proposal_id))
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
                            Text::new("proof-of-reserve")
                                .width(Length::Fixed(125.0))
                                .view(),
                        )
                        .push(Text::new(message).width(Length::Fill).view())
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .on_press(Message::View(Stage::CompletedProposal(
                                    *completed_proposal_id,
                                )))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                };
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
