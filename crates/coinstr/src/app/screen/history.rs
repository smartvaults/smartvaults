// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_sdk::core::proposal::CompletedProposal;
use coinstr_sdk::nostr::EventId;
use iced::widget::{Column, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::{CompletedProposalsList, Dashboard};
use crate::app::{Context, Message, State};
use crate::component::{Button, ButtonStyle, Text};
use crate::theme::icon::RELOAD;

#[derive(Debug, Clone)]
pub enum HistoryMessage {
    LoadCompletedProposals(BTreeMap<EventId, (EventId, CompletedProposal)>),
    Reload,
}

#[derive(Debug, Default)]
pub struct HistoryState {
    loading: bool,
    loaded: bool,
    proposals: BTreeMap<EventId, (EventId, CompletedProposal)>,
}

impl HistoryState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for HistoryState {
    fn title(&self) -> String {
        String::from("History")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move { client.get_completed_proposals().unwrap() },
            |p| HistoryMessage::LoadCompletedProposals(p).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::History(msg) = message {
            match msg {
                HistoryMessage::LoadCompletedProposals(proposals) => {
                    self.proposals = proposals;
                    self.loading = false;
                    self.loaded = true;
                    Command::none()
                }
                HistoryMessage::Reload => self.load(ctx),
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
                    .push(Text::new("No history").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .icon(RELOAD)
                            .text("Reload")
                            .width(Length::Fixed(250.0))
                            .on_press(HistoryMessage::Reload.into())
                            .view(),
                    )
                    .align_items(Alignment::Center);
            } else {
                center_y = false;
                content = content.push(CompletedProposalsList::new(self.proposals.clone()).view());
            }
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
    }
}

impl From<HistoryState> for Box<dyn State> {
    fn from(s: HistoryState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<HistoryMessage> for Message {
    fn from(msg: HistoryMessage) -> Self {
        Self::History(msg)
    }
}
