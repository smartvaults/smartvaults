// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::time::Duration;

use iced::alignment::Horizontal;
use iced::widget::{Column, Row};
use iced::{time, Alignment, Command, Element, Length, Subscription};
use smartvaults_sdk::nostr::{RelayStatus, Url};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Circle, Text};
use crate::theme::color::{BLACK, GREEN, GREY, RED, YELLOW};
use crate::theme::icon::{FULLSCREEN, PLUS, RELOAD, TRASH};

#[derive(Debug, Clone)]
pub struct Relay {
    url: Url,
    status: RelayStatus,
    queue: usize,
}

#[derive(Debug, Clone)]
pub enum RelaysMessage {
    LoadRelays(Vec<Relay>),
    RefreshRelays,
    RemoveRelay(Url),
    ErrorChanged(Option<String>),
}

#[derive(Debug, Default)]
pub struct RelaysState {
    loading: bool,
    loaded: bool,
    relays: Vec<Relay>,
    error: Option<String>,
}

impl RelaysState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for RelaysState {
    fn title(&self) -> String {
        String::from("Relays")
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(10)).map(|_| RelaysMessage::RefreshRelays.into())
        ])
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let mut relays = Vec::new();
                for (url, relay) in client.relays().await.into_iter() {
                    relays.push(Relay {
                        url,
                        status: relay.status().await,
                        queue: relay.queue(),
                    });
                }
                relays
            },
            |r| RelaysMessage::LoadRelays(r).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Relays(msg) = message {
            match msg {
                RelaysMessage::LoadRelays(relays) => {
                    self.relays = relays;
                    self.loading = false;
                    self.loaded = true;
                }
                RelaysMessage::RemoveRelay(url) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.remove_relay(url).await },
                        |res| match res {
                            Ok(_) => RelaysMessage::RefreshRelays.into(),
                            Err(e) => RelaysMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                RelaysMessage::ErrorChanged(e) => {
                    self.error = e;
                    self.loading = false;
                }
                RelaysMessage::RefreshRelays => return self.load(ctx),
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        if self.loaded {
            content = content
                .push(
                    Row::new()
                        .push(Text::new("Url").bold().big().width(Length::Fill).view())
                        .push(
                            Text::new("Status")
                                .bold()
                                .big()
                                .horizontal_alignment(Horizontal::Center)
                                .width(Length::Fixed(100.0))
                                .view(),
                        )
                        .push(
                            Text::new("Queue")
                                .bold()
                                .big()
                                .horizontal_alignment(Horizontal::Center)
                                .width(Length::Fixed(80.0))
                                .view(),
                        )
                        .push(
                            Button::new()
                                .icon(PLUS)
                                .style(ButtonStyle::Bordered)
                                .on_press(Message::View(Stage::AddRelay))
                                .loading(self.loading)
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .push(
                            Button::new()
                                .icon(RELOAD)
                                .style(ButtonStyle::Bordered)
                                .on_press(RelaysMessage::RefreshRelays.into())
                                .loading(self.loading)
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                )
                .push(rule::horizontal_bold());

            for Relay { url, status, queue } in self.relays.iter() {
                let status = match status {
                    RelayStatus::Initialized => Circle::new(7.0).color(GREY),
                    RelayStatus::Connecting => Circle::new(7.0).color(YELLOW),
                    RelayStatus::Connected => Circle::new(7.0).color(GREEN),
                    RelayStatus::Disconnected => Circle::new(7.0).color(RED),
                    RelayStatus::Stopped => Circle::new(7.0).color(BLACK),
                    RelayStatus::Terminated => continue,
                };

                let row = Row::new()
                    .push(Text::new(url.to_string()).width(Length::Fill).view())
                    .push(
                        Column::new()
                            .push(status)
                            .align_items(Alignment::Center)
                            .width(Length::Fixed(100.0)),
                    )
                    .push(
                        Text::new(queue.to_string())
                            .horizontal_alignment(Horizontal::Center)
                            .width(Length::Fixed(80.0))
                            .view(),
                    )
                    .push(
                        Button::new()
                            .icon(TRASH)
                            .on_press(RelaysMessage::RemoveRelay(url.clone()).into())
                            .loading(self.loading)
                            .style(ButtonStyle::BorderedDanger)
                            .width(Length::Fixed(40.0))
                            .view(),
                    )
                    .push(
                        Button::new()
                            .icon(FULLSCREEN)
                            .width(Length::Fixed(40.0))
                            .view(),
                    )
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill);
                content = content.push(row).push(rule::horizontal());
            }
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, false)
    }
}

impl From<RelaysState> for Box<dyn State> {
    fn from(s: RelaysState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<RelaysMessage> for Message {
    fn from(msg: RelaysMessage) -> Self {
        Self::Relays(msg)
    }
}
