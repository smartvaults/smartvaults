// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};

use coinstr_sdk::nostr::bitcoin::XOnlyPublicKey;
use coinstr_sdk::nostr::{EventId, Metadata};
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::constants::APP_NAME;
use crate::theme::color::RED;
use crate::theme::icon::{PLUS, RELOAD};

#[derive(Debug, Clone)]
pub enum ShareSignerMessage {
    LoadContacts(BTreeMap<XOnlyPublicKey, Metadata>),
    AddPublicKey(XOnlyPublicKey),
    RemovePublicKey(XOnlyPublicKey),
    Share,
    Reload,
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct ShareSignerState {
    loading: bool,
    loaded: bool,
    signer_id: EventId,
    contacts: BTreeMap<XOnlyPublicKey, Metadata>,
    public_keys: HashSet<XOnlyPublicKey>,
    error: Option<String>,
}

impl ShareSignerState {
    pub fn new(signer_id: EventId) -> Self {
        Self {
            loading: false,
            loaded: false,
            signer_id,
            contacts: BTreeMap::new(),
            public_keys: HashSet::new(),
            error: None,
        }
    }
}

impl State for ShareSignerState {
    fn title(&self) -> String {
        format!(
            "{APP_NAME} - Share Signer #{}",
            util::cut_event_id(self.signer_id)
        )
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(async move { client.get_contacts().unwrap() }, |p| {
            ShareSignerMessage::LoadContacts(p).into()
        })
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::ShareSigner(msg) = message {
            match msg {
                ShareSignerMessage::LoadContacts(contacts) => {
                    self.contacts = contacts;
                    self.loading = false;
                    self.loaded = true;
                }
                ShareSignerMessage::AddPublicKey(public_key) => {
                    self.public_keys.insert(public_key);
                }
                ShareSignerMessage::RemovePublicKey(public_key) => {
                    self.public_keys.remove(&public_key);
                }
                ShareSignerMessage::Share => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let signer_id = self.signer_id;
                    let public_keys = self.public_keys.iter().copied().collect();
                    return Command::perform(
                        async move {
                            client
                                .share_signer_to_multiple_public_keys(signer_id, public_keys)
                                .await
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Signers),
                            Err(e) => ShareSignerMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ShareSignerMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                ShareSignerMessage::Reload => return self.load(ctx),
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            if self.contacts.is_empty() {
                let add_contact_btn = button::primary_with_icon(PLUS, "Add contacts")
                    .on_press(Message::View(Stage::AddContact))
                    .width(Length::Fixed(250.0));
                let reload_btn = button::border_with_icon(RELOAD, "Reload")
                    .width(Length::Fixed(250.0))
                    .on_press(ShareSignerMessage::Reload.into());
                content = content
                    .push(Text::new("No contacts").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(add_contact_btn)
                    .push(reload_btn)
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                content = content
                    .push(
                        Text::new(format!(
                            "Share Signer #{}",
                            util::cut_event_id(self.signer_id)
                        ))
                        .size(40)
                        .bold()
                        .view(),
                    )
                    .push(Space::with_height(Length::Fixed(40.0)))
                    .push(
                        Row::new()
                            .push(
                                Text::new("Public Key")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(Text::new("Name").bold().bigger().width(Length::Fill).view())
                            .push(
                                Text::new("Display Name")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(Space::with_width(Length::Fixed(180.0)))
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (public_key, metadata) in self.contacts.iter() {
                    let select_btn = if self.public_keys.contains(public_key) {
                        button::primary("Selected")
                            .on_press(ShareSignerMessage::RemovePublicKey(*public_key).into())
                    } else {
                        button::border("Select")
                            .on_press(ShareSignerMessage::AddPublicKey(*public_key).into())
                    };

                    let row = Row::new()
                        .push(
                            Text::new(util::cut_public_key(*public_key))
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(metadata.name.as_deref().unwrap_or_default())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(metadata.display_name.as_deref().unwrap_or_default())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(select_btn.width(Length::Fixed(180.0)))
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill);
                    content = content.push(row).push(rule::horizontal());
                }

                if let Some(error) = &self.error {
                    content = content
                        .push(Space::with_height(Length::Fixed(20.0)))
                        .push(Text::new(error).color(RED).view());
                };

                content = content.push(Space::with_height(Length::Fixed(20.0))).push(
                    Column::new()
                        .push(
                            button::primary("Share")
                                .width(Length::Fixed(180.0))
                                .on_press(ShareSignerMessage::Share.into()),
                        )
                        .width(Length::Fill)
                        .align_items(Alignment::End),
                );
            }
        } else {
            content = content.push(Text::new("Loading...").view());
        }

        Dashboard::new().view(ctx, content, true, center_y)
    }
}

impl From<ShareSignerState> for Box<dyn State> {
    fn from(s: ShareSignerState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ShareSignerMessage> for Message {
    fn from(msg: ShareSignerMessage) -> Self {
        Self::ShareSigner(msg)
    }
}
