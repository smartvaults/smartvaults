// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bitcoin::XOnlyPublicKey;
use coinstr_core::nostr_sdk::EventId;
use coinstr_core::proposal::CompletedProposal;
use coinstr_core::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Text};
use crate::constants::APP_NAME;

#[derive(Debug, Clone)]
pub enum CompletedProposalMessage {
    Delete,
}

#[derive(Debug)]
pub struct CompletedProposalState {
    completed_proposal_id: EventId,
    completed_proposal: CompletedProposal,
    policy_id: EventId,
}

impl CompletedProposalState {
    pub fn new(
        completed_proposal_id: EventId,
        completed_proposal: CompletedProposal,
        policy_id: EventId,
    ) -> Self {
        Self {
            completed_proposal_id,
            completed_proposal,
            policy_id,
        }
    }
}

impl State for CompletedProposalState {
    fn title(&self) -> String {
        format!(
            "{APP_NAME} - Finalized proposal #{}",
            util::cut_event_id(self.completed_proposal_id)
        )
    }

    fn update(&mut self, _ctx: &mut Context, message: Message) -> Command<Message> {
        if let Message::CompletedProposal(msg) = message {
            match msg {
                CompletedProposalMessage::Delete => {}
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new()
            .spacing(10)
            .padding(20)
            .push(
                Text::new(format!(
                    "Finalized proposal #{}",
                    util::cut_event_id(self.completed_proposal_id)
                ))
                .size(40)
                .bold()
                .view(),
            )
            .push(Space::with_height(Length::Fixed(40.0)))
            .push(
                Text::new(format!("Policy ID: {}", util::cut_event_id(self.policy_id)))
                    .on_press(Message::View(Stage::Policy(self.policy_id)))
                    .view(),
            );

        let approvals = match &self.completed_proposal {
            CompletedProposal::Spending {
                txid,
                description,
                approvals,
            } => {
                content = content
                    .push(Text::new("Type: spending").view())
                    .push(
                        Text::new(format!("Txid: {txid}"))
                            .on_press(Message::View(Stage::Transaction(*txid)))
                            .view(),
                    )
                    .push(Text::new(format!("Description: {description}")).view());

                approvals
            }
            CompletedProposal::ProofOfReserve {
                message, approvals, ..
            } => {
                content = content
                    .push(Text::new("Type: proof-of-reserve").view())
                    .push(Text::new(format!("Message: {message}")).view());

                approvals
            }
        };

        if !approvals.is_empty() {
            content = content
                .push(Space::with_height(20.0))
                .push(Text::new("Approvals").bold().bigger().view())
                .push(Space::with_height(10.0))
                .push(
                    Row::new()
                        .push(Text::new("User").bold().bigger().width(Length::Fill).view())
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                )
                .push(rule::horizontal_bold());

            for author in approvals.iter() {
                let row = Row::new()
                    .push(
                        Text::new(cut_public_key(*author))
                            .width(Length::Fill)
                            .view(),
                    )
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill);
                content = content.push(row).push(rule::horizontal());
            }
        }

        Dashboard::new().view(ctx, content, false, false)
    }
}

impl From<CompletedProposalState> for Box<dyn State> {
    fn from(s: CompletedProposalState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<CompletedProposalMessage> for Message {
    fn from(msg: CompletedProposalMessage) -> Self {
        Self::CompletedProposal(msg)
    }
}

fn cut_public_key(pk: XOnlyPublicKey) -> String {
    let pk = pk.to_string();
    format!("{}:{}", &pk[0..8], &pk[pk.len() - 8..])
}
