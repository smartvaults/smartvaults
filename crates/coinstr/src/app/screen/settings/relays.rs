// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::time::Duration;

use coinstr_sdk::nostr::relay::RelayConnectionStats;
use coinstr_sdk::nostr::{RelayStatus, Url};
use iced::widget::{Column, Row, Space};
use iced::{time, Alignment, Command, Element, Length, Subscription};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, State};
use crate::component::{button, rule, Circle, Text};
use crate::constants::APP_NAME;
use crate::theme::color::{BLACK, GREEN, GREY, RED, YELLOW};
use crate::theme::icon::RELOAD;

#[derive(Debug, Clone)]
pub struct Relay {
    url: Url,
    status: RelayStatus,
    stats: RelayConnectionStats,
}

#[derive(Debug, Clone)]
pub enum RelaysMessage {
    LoadRelays(Vec<Relay>),
    RefreshRelays,
}

#[derive(Debug, Default)]
pub struct RelaysState {
    loading: bool,
    loaded: bool,
    relays: Vec<Relay>,
}

impl RelaysState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for RelaysState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Relays")
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
                        stats: relay.stats(),
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
                RelaysMessage::RefreshRelays => return self.load(ctx),
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            center_y = false;

            let mut reload_btn = button::border_only_icon(RELOAD).width(Length::Fixed(40.0));

            if !self.loading {
                reload_btn = reload_btn.on_press(RelaysMessage::RefreshRelays.into());
            }

            content = content
                .push(
                    Row::new()
                        .push(Text::new("Url").bold().bigger().width(Length::Fill).view())
                        .push(
                            Text::new("Status")
                                .bold()
                                .bigger()
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new("Attemps")
                                .bold()
                                .bigger()
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new("Success")
                                .bold()
                                .bigger()
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new("Connected at")
                                .bold()
                                .bigger()
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(reload_btn)
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill),
                )
                .push(rule::horizontal_bold());

            for Relay {
                url, status, stats, ..
            } in self.relays.iter()
            {
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
                    .push(Row::new().push(status).width(Length::Fill))
                    .push(
                        Text::new(stats.attempts().to_string())
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(
                        Text::new(stats.success().to_string())
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(
                        Text::new(stats.connected_at().to_human_datetime())
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(Space::with_width(Length::Fixed(40.0)))
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill);
                content = content.push(row).push(rule::horizontal());
            }
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, true, center_y)
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
