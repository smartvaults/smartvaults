// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_sdk::core::proposal::Proposal;
use coinstr_sdk::nostr::EventId;
use iced::widget::{Column, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::{Dashboard, PendingProposalsList};
use crate::app::{Context, Message, State};
use crate::component::{Button, ButtonStyle, Text};
use crate::theme::icon::RELOAD;

#[derive(Debug, Clone)]
pub enum ProposalsMessage {
    LoadProposals(BTreeMap<EventId, (EventId, Proposal)>),
    Reload,
}

#[derive(Debug, Default)]
pub struct ProposalsState {
    loading: bool,
    loaded: bool,
    proposals: BTreeMap<EventId, (EventId, Proposal)>,
}

impl ProposalsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ProposalsState {
    fn title(&self) -> String {
        String::from("Proposals")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(async move { client.get_proposals().unwrap() }, |p| {
            ProposalsMessage::LoadProposals(p).into()
        })
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Proposals(msg) = message {
            match msg {
                ProposalsMessage::LoadProposals(proposals) => {
                    self.proposals = proposals;
                    self.loading = false;
                    self.loaded = true;
                    Command::none()
                }
                ProposalsMessage::Reload => self.load(ctx),
            }
        } else {
            Command::none()
        }
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            if self.proposals.is_empty() {
                content = content
                    .push(Text::new("No proposals").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .icon(RELOAD)
                            .text("Reload")
                            .width(Length::Fixed(250.0))
                            .on_press(ProposalsMessage::Reload.into())
                            .view(),
                    )
                    .align_items(Alignment::Center);
            } else {
                center_y = false;
                content = content.push(PendingProposalsList::new(self.proposals.clone()).view());
            }
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
    }
}

impl From<ProposalsState> for Box<dyn State> {
    fn from(s: ProposalsState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ProposalsMessage> for Message {
    fn from(msg: ProposalsMessage) -> Self {
        Self::Proposals(msg)
    }
}
