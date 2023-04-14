// Copyright (c) 2022 Yuki Kishimoto
// Distributed under the MIT software license

use coinstr_core::nostr_sdk::EventId;
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::constants::APP_NAME;
use crate::theme::icon::{FULLSCREEN, PLUS, RELOAD};

#[derive(Debug, Clone)]
pub enum ProposalsMessage {
    LoadProposals(Vec<(EventId, SpendingProposal)>),
    Reload,
}

#[derive(Debug, Default)]
pub struct ProposalsState {
    loading: bool,
    loaded: bool,
    proposals: Vec<(EventId, SpendingProposal)>,
}

impl ProposalsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ProposalsState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Proposals")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let cache = ctx.cache.clone();
        Command::perform(async move { cache.get_proposals().unwrap() }, |p| {
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
                let reload_btn = button::border_with_icon(RELOAD, "Reload")
                    .width(Length::Fixed(250.0))
                    .on_press(ProposalsMessage::Reload.into());
                content = content
                    .push(Text::new("No proposals").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(reload_btn)
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                let new_spengind_proposal_btn =
                    button::border_only_icon(PLUS).width(Length::Fixed(40.0));
                let mut reload_btn = button::border_only_icon(RELOAD).width(Length::Fixed(40.0));

                if !self.loading {
                    reload_btn = reload_btn.on_press(ProposalsMessage::Reload.into());
                }

                content = content
                    .push(
                        Row::new()
                            .push(
                                Text::new("Proposal ID")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new("Policy ID")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(Text::new("Name").bold().bigger().width(Length::Fill).view())
                            .push(
                                Text::new("Description")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(new_spengind_proposal_btn)
                            .push(reload_btn)
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (proposal_id, proposal) in self.proposals.iter() {
                    let row = Row::new()
                        .push(
                            Text::new(util::cut_event_id(*proposal_id))
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(&proposal.amount.to_string())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(Text::new(&proposal.memo).width(Length::Fill).view())
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
                    content = content.push(row).push(rule::horizontal());
                }
            }
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, true, center_y)
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
