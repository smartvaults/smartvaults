// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::time::Duration;

use coinstr_sdk::core::bitcoin::XOnlyPublicKey;
use coinstr_sdk::db::model::NostrConnectRequest;
use coinstr_sdk::nostr::nips::nip46::NostrConnectURI;
use coinstr_sdk::nostr::{EventId, Timestamp};
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

pub mod add_session;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::theme::color::RED;
use crate::theme::icon::{CHECK, FULLSCREEN, PLUS, RELOAD, STOPWATCH, TRASH};

#[derive(Debug, Clone)]
pub enum ConnectMessage {
    Load(
        (
            Vec<(NostrConnectURI, Timestamp)>,
            BTreeMap<EventId, NostrConnectRequest>,
        ),
    ),
    ApproveRequest(EventId),
    DeleteRequest(EventId),
    DisconnectSession(XOnlyPublicKey),
    ErrorChanged(Option<String>),
    Reload,
}

#[derive(Debug, Default)]
pub struct ConnectState {
    loading: bool,
    loaded: bool,
    sessions: Vec<(NostrConnectURI, Timestamp)>,
    requests: BTreeMap<EventId, NostrConnectRequest>,
    error: Option<String>,
}

impl ConnectState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ConnectState {
    fn title(&self) -> String {
        String::from("Connect")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let sessions = client.get_nostr_connect_sessions().unwrap();
                let requests = client.get_nostr_connect_requests(false).unwrap();
                (sessions, requests)
            },
            |c| ConnectMessage::Load(c).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Connect(msg) = message {
            match msg {
                ConnectMessage::Load((sessions, requests)) => {
                    self.sessions = sessions;
                    self.requests = requests;
                    self.loading = false;
                    self.loaded = true;
                    Command::none()
                }
                ConnectMessage::ApproveRequest(id) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    Command::perform(
                        async move { client.approve_nostr_connect_request(id).await },
                        |res| match res {
                            Ok(_) => ConnectMessage::Reload.into(),
                            Err(e) => ConnectMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    )
                }
                ConnectMessage::DeleteRequest(id) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    Command::perform(
                        async move { client.delete_nostr_connect_request(id) },
                        |res| match res {
                            Ok(_) => ConnectMessage::Reload.into(),
                            Err(e) => ConnectMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    )
                }
                ConnectMessage::DisconnectSession(app_public_key) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    Command::perform(
                        async move {
                            client
                                .disconnect_nostr_connect_session(
                                    app_public_key,
                                    Some(Duration::from_secs(30)),
                                )
                                .await
                        },
                        |res| match res {
                            Ok(_) => ConnectMessage::Reload.into(),
                            Err(e) => ConnectMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    )
                }
                ConnectMessage::ErrorChanged(e) => {
                    self.loading = false;
                    self.error = e;
                    Command::none()
                }
                ConnectMessage::Reload => self.load(ctx),
            }
        } else {
            Command::none()
        }
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            if self.sessions.is_empty() {
                let add_session_btn = button::primary_with_icon(PLUS, "Add session")
                    .width(Length::Fixed(250.0))
                    .on_press(Message::View(Stage::AddNostrConnectSession));
                let reload_btn = button::border_with_icon(RELOAD, "Reload")
                    .width(Length::Fixed(250.0))
                    .on_press(ConnectMessage::Reload.into());
                content = content
                    .push(Text::new("No sessions").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(add_session_btn)
                    .push(reload_btn)
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                let add_session_btn = button::border_only_icon(PLUS)
                    .width(Length::Fixed(40.0))
                    .on_press(Message::View(Stage::AddNostrConnectSession));
                let mut reload_btn = button::border_only_icon(RELOAD).width(Length::Fixed(40.0));

                if !self.loading {
                    reload_btn = reload_btn.on_press(ConnectMessage::Reload.into());
                }

                // Sessions

                content = content
                    .push(Text::new("Sessions").bigger().bold().view())
                    .push(
                        Row::new()
                            .push(
                                Text::new("App Public Key")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fixed(175.0))
                                    .view(),
                            )
                            .push(
                                Text::new("App Name")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new("Relay Url")
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
                            .push(Space::with_width(Length::Fixed(40.0)))
                            .push(add_session_btn)
                            .push(reload_btn)
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (uri, timestamp) in self.sessions.iter() {
                    let mut disconnect_btn =
                        button::danger_border_only_icon(TRASH).width(Length::Fixed(40.0));

                    if !self.loading {
                        disconnect_btn = disconnect_btn
                            .on_press(ConnectMessage::DisconnectSession(uri.public_key).into());
                    }

                    let row = Row::new()
                        .push(
                            Text::new(util::cut_public_key(uri.public_key))
                                .width(Length::Fixed(175.0))
                                .view(),
                        )
                        .push(
                            Text::new(uri.metadata.name.clone())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(uri.relay_url.to_string())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(timestamp.to_human_datetime())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(button::border_only_icon(STOPWATCH).width(Length::Fixed(40.0)))
                        .push(disconnect_btn)
                        .push(button::primary_only_icon(FULLSCREEN).width(Length::Fixed(40.0)))
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill);
                    content = content.push(row).push(rule::horizontal());
                }

                if let Some(e) = &self.error {
                    content = content.push(Text::new(e).color(RED).view());
                }

                // Requests

                if !self.requests.is_empty() {
                    content = content
                        .push(Space::with_height(Length::Fixed(40.0)))
                        .push(Text::new("Pending requests").bigger().bold().view())
                        .push(
                            Row::new()
                                .push(
                                    Text::new("ID")
                                        .bold()
                                        .bigger()
                                        .width(Length::Fixed(115.0))
                                        .view(),
                                )
                                .push(
                                    Text::new("App Public Key")
                                        .bold()
                                        .bigger()
                                        .width(Length::Fixed(175.0))
                                        .view(),
                                )
                                .push(
                                    Text::new("Method")
                                        .bold()
                                        .bigger()
                                        .width(Length::Fill)
                                        .view(),
                                )
                                .push(
                                    Text::new("Requested at")
                                        .bold()
                                        .bigger()
                                        .width(Length::Fill)
                                        .view(),
                                )
                                .push(Space::with_width(Length::Fixed(120.0)))
                                .push(Space::with_width(Length::Fixed(40.0)))
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill),
                        )
                        .push(rule::horizontal_bold());

                    for (req_id, request) in self.requests.iter() {
                        if let Ok(req) = request.message.to_request() {
                            let mut approve_btn =
                                button::border_only_icon(CHECK).width(Length::Fixed(120.0));
                            let mut delete_btn =
                                button::danger_border_only_icon(TRASH).width(Length::Fixed(40.0));

                            if !self.loading {
                                approve_btn = approve_btn
                                    .on_press(ConnectMessage::ApproveRequest(*req_id).into());
                                delete_btn = delete_btn
                                    .on_press(ConnectMessage::DeleteRequest(*req_id).into());
                            }

                            let row = Row::new()
                                .push(
                                    Text::new(util::cut_event_id(*req_id))
                                        .width(Length::Fixed(115.0))
                                        .view(),
                                )
                                .push(
                                    Text::new(util::cut_public_key(request.app_public_key))
                                        .width(Length::Fixed(175.0))
                                        .view(),
                                )
                                .push(Text::new(req.method()).width(Length::Fill).view())
                                .push(
                                    Text::new(request.timestamp.to_human_datetime())
                                        .width(Length::Fill)
                                        .view(),
                                )
                                .push(approve_btn)
                                .push(delete_btn)
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill);
                            content = content.push(row).push(rule::horizontal());
                        }
                    }
                }
            }
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, true, center_y)
    }
}

impl From<ConnectState> for Box<dyn State> {
    fn from(s: ConnectState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ConnectMessage> for Message {
    fn from(msg: ConnectMessage) -> Self {
        Self::Connect(msg)
    }
}
