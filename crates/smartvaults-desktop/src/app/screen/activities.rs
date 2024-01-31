// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeSet;

use iced::widget::{Column, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::types::{GetProposal, GetTransaction};

use crate::app::component::{Activity, Dashboard};
use crate::app::{Context, Message, State};
use crate::component::{Button, ButtonStyle, Text};
use crate::theme::icon::RELOAD;

#[derive(Debug, Clone)]
pub enum ActivityMessage {
    Load(Vec<GetProposal>, BTreeSet<GetTransaction>),
    Reload,
}

#[derive(Debug, Default)]
pub struct ActivityState {
    loading: bool,
    loaded: bool,
    proposals: Vec<GetProposal>,
    txs: BTreeSet<GetTransaction>,
}

impl ActivityState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ActivityState {
    fn title(&self) -> String {
        String::from("Proposals")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let proposals = client.proposals().await.unwrap();
                let txs = client.get_all_transactions().await.unwrap();
                (proposals, txs)
            },
            |(proposals, txs)| ActivityMessage::Load(proposals, txs).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Activity(msg) = message {
            match msg {
                ActivityMessage::Load(proposals, txs) => {
                    self.proposals = proposals;
                    self.txs = txs;
                    self.loading = false;
                    self.loaded = true;
                    Command::none()
                }
                ActivityMessage::Reload => self.load(ctx),
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
                            .on_press(ActivityMessage::Reload.into())
                            .view(),
                    )
                    .align_items(Alignment::Center);
            } else {
                center_y = false;
                content =
                    content.push(Activity::new(self.proposals.clone(), self.txs.clone()).view(ctx));
            }
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
    }
}

impl From<ActivityState> for Box<dyn State> {
    fn from(s: ActivityState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ActivityMessage> for Message {
    fn from(msg: ActivityMessage) -> Self {
        Self::Activity(msg)
    }
}
