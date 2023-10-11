// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::time::Duration;

use iced::widget::{Column, Row, Space};
use iced::{time, Command, Element, Length, Subscription};
use smartvaults_sdk::nostr::relay::RelayConnectionStats;
use smartvaults_sdk::nostr::{RelayStatus, Url};
use smartvaults_sdk::util::format;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::Text;

#[derive(Debug, Clone)]
pub struct Relay {
    status: RelayStatus,
    stats: RelayConnectionStats,
    latency: Option<Duration>,
    queue: usize,
}

#[derive(Debug, Clone)]
pub enum RelayMessage {
    LoadRelay(Relay),
    RefreshRelays,
    RemoveRelay(Url),
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct RelayState {
    loading: bool,
    loaded: bool,
    url: Url,
    relay: Option<Relay>,
    error: Option<String>,
}

impl RelayState {
    pub fn new(url: Url) -> Self {
        Self {
            loading: false,
            loaded: false,
            url,
            relay: None,
            error: None,
        }
    }
}

impl State for RelayState {
    fn title(&self) -> String {
        format!("Relay {}", self.url)
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(10)).map(|_| RelayMessage::RefreshRelays.into())
        ])
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let url = self.url.clone();
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let relay = client.relay(url).await?;
                let stats = relay.stats();
                let relay = Relay {
                    status: relay.status().await,
                    latency: stats.latency().await,
                    stats,
                    queue: relay.queue(),
                };
                Ok::<Relay, Box<dyn std::error::Error>>(relay)
            },
            |res| match res {
                Ok(r) => RelayMessage::LoadRelay(r).into(),
                Err(e) => {
                    tracing::error!("Impossible to load relay: {e}");
                    Message::View(Stage::Relays)
                }
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Relay(msg) = message {
            match msg {
                RelayMessage::LoadRelay(relay) => {
                    self.relay = Some(relay);
                    self.loading = false;
                    self.loaded = true;
                }
                RelayMessage::RemoveRelay(url) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.remove_relay(url).await },
                        |res| match res {
                            Ok(_) => RelayMessage::RefreshRelays.into(),
                            Err(e) => RelayMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                RelayMessage::ErrorChanged(e) => {
                    self.error = e;
                    self.loading = false;
                }
                RelayMessage::RefreshRelays => return self.load(ctx),
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(20).padding(20);

        if let Some(Relay {
            status,
            stats,
            latency,
            queue,
        }) = &self.relay
        {
            content = content
                .push(Text::new(self.title()).size(40).bold().view())
                .push(Space::with_height(Length::Fixed(10.0)))
                .push(
                    Row::new()
                        .push(
                            Column::new()
                                .push(Text::new("Status").big().extra_light().view())
                                .push(Text::new(status.to_string()).big().view())
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Attempts").big().extra_light().view())
                                .push(
                                    Text::new(format::number(stats.attempts() as u64))
                                        .big()
                                        .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Success").big().extra_light().view())
                                .push(
                                    Text::new(format::number(stats.success() as u64))
                                        .big()
                                        .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(
                    Row::new()
                        .push(
                            Column::new()
                                .push(Text::new("Latency").big().extra_light().view())
                                .push(
                                    Text::new(match latency {
                                        Some(latency) => format!("{} ms", latency.as_millis()),
                                        None => String::from("-"),
                                    })
                                    .big()
                                    .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Bytes sent").big().extra_light().view())
                                .push(
                                    Text::new(format::big_number(stats.bytes_sent() as u64))
                                        .big()
                                        .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Bytes received").big().extra_light().view())
                                .push(
                                    Text::new(format::big_number(stats.bytes_received() as u64))
                                        .big()
                                        .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(
                    Row::new()
                        .push(
                            Column::new()
                                .push(Text::new("Queue").big().extra_light().view())
                                .push(Text::new(format::number(*queue as u64)).big().view())
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Connected at").big().extra_light().view())
                                .push(
                                    Text::new(stats.connected_at().to_human_datetime())
                                        .big()
                                        .view(),
                                )
                                .spacing(10)
                                .width(Length::FillPortion(2)),
                        ),
                )
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, false)
    }
}

impl From<RelayState> for Box<dyn State> {
    fn from(s: RelayState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<RelayMessage> for Message {
    fn from(msg: RelayMessage) -> Self {
        Self::Relay(msg)
    }
}
