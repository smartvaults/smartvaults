// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::time::Duration;

use coinstr_core::nostr_sdk::EventId;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::util;
use iced::widget::{Column, Row, Space};
use iced::{Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{button, Text};
use crate::constants::APP_NAME;
use crate::theme::color::DARK_RED;

#[derive(Debug, Clone)]
pub enum ProposalMessage {
    Approve,
    Approved(Option<EventId>),
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct ProposalState {
    loading: bool,
    loaded: bool,
    approved: Option<EventId>,
    proposal_id: EventId,
    proposal: SpendingProposal,
    error: Option<String>,
}

impl ProposalState {
    pub fn new(proposal_id: EventId, proposal: SpendingProposal) -> Self {
        Self {
            loading: false,
            loaded: false,
            approved: None,
            proposal_id,
            proposal,
            error: None,
        }
    }
}

impl State for ProposalState {
    fn title(&self) -> String {
        format!(
            "{APP_NAME} - Proposal #{}",
            util::cut_event_id(self.proposal_id)
        )
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        let proposal_id = self.proposal_id;
        self.loading = true;
        Command::perform(
            async move {
                client
                    .get_approved_proposal_by_id_for_own_keys(
                        proposal_id,
                        Some(Duration::from_secs(60)),
                    )
                    .await
                    .ok()
            },
            |result| ProposalMessage::Approved(result).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Proposal(msg) = message {
            match msg {
                ProposalMessage::Approved(value) => {
                    self.approved = value;
                    self.loading = false;
                    self.loaded = true;
                }
                ProposalMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                ProposalMessage::Approve => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let proposal_id = self.proposal_id;
                    return Command::perform(
                        async move { client.approve(proposal_id, None).await },
                        |res| match res {
                            Ok(event_id) => ProposalMessage::Approved(Some(event_id)).into(),
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;
        let mut center_x = true;

        if self.loaded {
            center_y = false;
            center_x = false;
            let title = format!("Proposal #{}", util::cut_event_id(self.proposal_id));
            content = content
                .push(Text::new(title).size(40).bold().view())
                .push(Space::with_height(Length::Fixed(40.0)))
                .push(Text::new(format!("Address: {}", self.proposal.to_address)).view())
                .push(
                    Text::new(format!(
                        "Amount: {} sats",
                        util::format::number(self.proposal.amount)
                    ))
                    .view(),
                )
                .push(Text::new(format!("Memo: {}", &self.proposal.memo)).view());

            match self.approved {
                Some(event_id) => {
                    let approve_btn = button::border("Approve");
                    let broadcast_btn = button::primary("Broadcast");
                    content =
                        content.push(Row::new().push(approve_btn).push(broadcast_btn).spacing(10));

                    let msg = format!("Proposal approved with event {event_id}");
                    content = content.push(Text::new(msg).view());
                }
                None => {
                    let mut approve_btn = button::primary("Approve");
                    let broadcast_btn = button::border("Broadcast");

                    if !self.loading {
                        approve_btn = approve_btn.on_press(ProposalMessage::Approve.into());
                    }

                    content =
                        content.push(Row::new().push(approve_btn).push(broadcast_btn).spacing(10));
                }
            }

            if let Some(error) = &self.error {
                content = content.push(Text::new(error).color(DARK_RED).view());
            };
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, center_x, center_y)
    }
}

impl From<ProposalState> for Box<dyn State> {
    fn from(s: ProposalState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ProposalMessage> for Message {
    fn from(msg: ProposalMessage) -> Self {
        Self::Proposal(msg)
    }
}
