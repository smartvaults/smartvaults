// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::time::Duration;

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::secp256k1::XOnlyPublicKey;
use smartvaults_sdk::nostr::nips::nip46::NostrConnectURI;
use smartvaults_sdk::nostr::{EventId, Timestamp};
use smartvaults_sdk::types::NostrConnectRequest;
use smartvaults_sdk::util;

pub mod add_session;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::color::RED;
use crate::theme::icon::{CHECK, FULLSCREEN, PLUS, RELOAD, STOP, STOPWATCH, TRASH};

type Sessions = Vec<(NostrConnectURI, Timestamp)>;
type Requests = Vec<NostrConnectRequest>;
type Authorizations = BTreeMap<XOnlyPublicKey, Timestamp>;

#[derive(Debug, Clone)]
pub enum ConnectMessage {
    Load((Sessions, Requests, Requests, Authorizations)),
    ApproveRequest(EventId),
    DeleteRequest(EventId),
    DisconnectSession(XOnlyPublicKey),
    AddAuthorization(XOnlyPublicKey),
    RevokeAuthorization(XOnlyPublicKey),
    ErrorChanged(Option<String>),
    Reload,
}

#[derive(Debug, Default)]
pub struct ConnectState {
    loading: bool,
    loaded: bool,
    sessions: Sessions,
    pending_requests: Requests,
    approved_requests: Requests,
    authorizations: Authorizations,
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
                let sessions = client.get_nostr_connect_sessions().await.unwrap();
                let pending_requests = client.get_nostr_connect_requests(false).await.unwrap();
                let approved_requests = client.get_nostr_connect_requests(true).await.unwrap();
                let authorizations = client.get_nostr_connect_pre_authorizations().await;
                (
                    sessions,
                    pending_requests,
                    approved_requests,
                    authorizations,
                )
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
                ConnectMessage::Load((
                    sessions,
                    pending_requests,
                    approved_requests,
                    authorizations,
                )) => {
                    self.sessions = sessions;
                    self.pending_requests = pending_requests;
                    self.approved_requests = approved_requests;
                    self.authorizations = authorizations;
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
                        async move { client.reject_nostr_connect_request(id).await },
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
                                .disconnect_nostr_connect_session(app_public_key)
                                .await
                        },
                        |res| match res {
                            Ok(_) => ConnectMessage::Reload.into(),
                            Err(e) => ConnectMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    )
                }
                ConnectMessage::AddAuthorization(public_key) => {
                    let client = ctx.client.clone();
                    Command::perform(
                        async move {
                            client
                                .auto_approve_nostr_connect_requests(
                                    public_key,
                                    Duration::from_secs(60 * 60),
                                )
                                .await
                        },
                        |_| ConnectMessage::Reload.into(),
                    )
                }
                ConnectMessage::RevokeAuthorization(public_key) => {
                    let client = ctx.client.clone();
                    Command::perform(
                        async move { client.revoke_nostr_connect_auto_approve(public_key).await },
                        |_| ConnectMessage::Reload.into(),
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
                content = content
                    .push(Text::new("No sessions").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .icon(PLUS)
                            .text("Add session")
                            .width(Length::Fixed(250.0))
                            .on_press(Message::View(Stage::AddNostrConnectSession))
                            .view(),
                    )
                    .push(
                        Button::new()
                            .icon(RELOAD)
                            .text("Reload")
                            .style(ButtonStyle::Bordered)
                            .width(Length::Fixed(250.0))
                            .on_press(ConnectMessage::Reload.into())
                            .view(),
                    )
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                // Sessions

                content = content
                    .push(Text::new("Sessions").big().bold().view())
                    .push(
                        Row::new()
                            .push(
                                Text::new("App Public Key")
                                    .bold()
                                    .big()
                                    .width(Length::Fixed(175.0))
                                    .view(),
                            )
                            .push(
                                Text::new("App Name")
                                    .bold()
                                    .big()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new("Relay Url")
                                    .bold()
                                    .big()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new("Connected at")
                                    .bold()
                                    .big()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new("Pre-authorized until")
                                    .bold()
                                    .big()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(Space::with_width(Length::Fixed(40.0)))
                            .push(
                                Button::new()
                                    .icon(PLUS)
                                    .width(Length::Fixed(40.0))
                                    .style(ButtonStyle::Bordered)
                                    .on_press(Message::View(Stage::AddNostrConnectSession))
                                    .view(),
                            )
                            .push(
                                Button::new()
                                    .icon(RELOAD)
                                    .on_press(ConnectMessage::Reload.into())
                                    .loading(self.loading)
                                    .style(ButtonStyle::Bordered)
                                    .width(Length::Fixed(40.0))
                                    .view(),
                            )
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (uri, timestamp) in self.sessions.iter() {
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
                        .push(
                            Text::new(match self.authorizations.get(&uri.public_key) {
                                Some(timestamp) => timestamp.to_human_datetime(),
                                None => String::from("-"),
                            })
                            .width(Length::Fill)
                            .view(),
                        )
                        .push(
                            if self.authorizations.get(&uri.public_key).is_some() {
                                Button::new()
                                    .icon(STOP)
                                    .style(ButtonStyle::BorderedDanger)
                                    .loading(self.loading)
                                    .on_press(
                                        ConnectMessage::RevokeAuthorization(uri.public_key).into(),
                                    )
                            } else {
                                Button::new()
                                    .icon(STOPWATCH)
                                    .style(ButtonStyle::Bordered)
                                    .loading(self.loading)
                                    .on_press(
                                        ConnectMessage::AddAuthorization(uri.public_key).into(),
                                    )
                            }
                            .width(Length::Fixed(40.0))
                            .view(),
                        )
                        .push(
                            Button::new()
                                .icon(TRASH)
                                .on_press(ConnectMessage::DisconnectSession(uri.public_key).into())
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

                if let Some(e) = &self.error {
                    content = content.push(Text::new(e).color(RED).view());
                }

                // Pending Requests

                if !self.pending_requests.is_empty() {
                    content = content
                        .push(Space::with_height(Length::Fixed(40.0)))
                        .push(Text::new("Pending requests").big().bold().view())
                        .push(
                            Row::new()
                                .push(
                                    Text::new("ID")
                                        .bold()
                                        .big()
                                        .width(Length::Fixed(115.0))
                                        .view(),
                                )
                                .push(
                                    Text::new("App Public Key")
                                        .bold()
                                        .big()
                                        .width(Length::Fixed(175.0))
                                        .view(),
                                )
                                .push(Text::new("Method").bold().big().width(Length::Fill).view())
                                .push(
                                    Text::new("Requested at")
                                        .bold()
                                        .big()
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

                    for request in self.pending_requests.iter() {
                        if let Ok(req) = request.message.to_request() {
                            let row = Row::new()
                                .push(
                                    Text::new(util::cut_event_id(request.event_id))
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
                                .push(
                                    Button::new()
                                        .icon(CHECK)
                                        .on_press(
                                            ConnectMessage::ApproveRequest(request.event_id).into(),
                                        )
                                        .loading(self.loading)
                                        .style(ButtonStyle::Bordered)
                                        .width(Length::Fixed(120.0))
                                        .view(),
                                )
                                .push(
                                    Button::new()
                                        .icon(TRASH)
                                        .on_press(
                                            ConnectMessage::DeleteRequest(request.event_id).into(),
                                        )
                                        .loading(self.loading)
                                        .style(ButtonStyle::BorderedDanger)
                                        .width(Length::Fixed(40.0))
                                        .view(),
                                )
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill);
                            content = content.push(row).push(rule::horizontal());
                        }
                    }
                }

                // Approved Requests

                if !self.approved_requests.is_empty() {
                    content = content
                        .push(Space::with_height(Length::Fixed(40.0)))
                        .push(Text::new("Approved requests").big().bold().view())
                        .push(
                            Row::new()
                                .push(
                                    Text::new("ID")
                                        .bold()
                                        .big()
                                        .width(Length::Fixed(115.0))
                                        .view(),
                                )
                                .push(
                                    Text::new("App Public Key")
                                        .bold()
                                        .big()
                                        .width(Length::Fixed(175.0))
                                        .view(),
                                )
                                .push(Text::new("Method").bold().big().width(Length::Fill).view())
                                .push(
                                    Text::new("Requested at")
                                        .bold()
                                        .big()
                                        .width(Length::Fill)
                                        .view(),
                                )
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill),
                        )
                        .push(rule::horizontal_bold());

                    for request in self.approved_requests.iter() {
                        if let Ok(req) = request.message.to_request() {
                            let row = Row::new()
                                .push(
                                    Text::new(util::cut_event_id(request.event_id))
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
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill);
                            content = content.push(row).push(rule::horizontal());
                        }
                    }
                }
            }
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
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
