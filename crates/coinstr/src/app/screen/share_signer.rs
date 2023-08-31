// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};

use coinstr_sdk::nostr::secp256k1::XOnlyPublicKey;
use coinstr_sdk::nostr::{EventId, Metadata};
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::color::RED;
use crate::theme::icon::{PLUS, RELOAD};

#[derive(Debug, Clone)]
pub enum ShareSignerMessage {
    Load(BTreeMap<XOnlyPublicKey, Metadata>, Vec<XOnlyPublicKey>),
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
    already_shared_with: Vec<XOnlyPublicKey>,
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
            already_shared_with: Vec::new(),
            error: None,
        }
    }
}

impl State for ShareSignerState {
    fn title(&self) -> String {
        format!("Share Signer #{}", util::cut_event_id(self.signer_id))
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        let signer_id = self.signer_id;
        Command::perform(
            async move {
                let contacts = client.get_contacts().await.unwrap();
                let mut already_shared_with: Vec<XOnlyPublicKey> = Vec::new();
                for public_key in contacts.keys() {
                    if client
                        .db
                        .my_shared_signer_already_shared(signer_id, *public_key)
                        .await
                        .unwrap()
                    {
                        already_shared_with.push(*public_key);
                    }
                }
                (contacts, already_shared_with)
            },
            |(contacts, already_shared_with)| {
                ShareSignerMessage::Load(contacts, already_shared_with).into()
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::ShareSigner(msg) = message {
            match msg {
                ShareSignerMessage::Load(contacts, already_shared_with) => {
                    self.contacts = contacts;
                    self.already_shared_with = already_shared_with;
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
                content = content
                    .push(Text::new("No contacts").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(
                        Button::new()
                            .icon(PLUS)
                            .text("Add contacts")
                            .on_press(Message::View(Stage::AddContact))
                            .width(Length::Fixed(250.0))
                            .view(),
                    )
                    .push(
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .icon(RELOAD)
                            .text("Reload")
                            .width(Length::Fixed(250.0))
                            .on_press(ShareSignerMessage::Reload.into())
                            .view(),
                    )
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
                                    .big()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(Text::new("Name").bold().big().width(Length::Fill).view())
                            .push(
                                Text::new("Display Name")
                                    .bold()
                                    .big()
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
                    let select_btn = if self.already_shared_with.contains(public_key) {
                        Button::new().text("Already shared")
                    } else if self.public_keys.contains(public_key) {
                        Button::new()
                            .text("Selected")
                            .on_press(ShareSignerMessage::RemovePublicKey(*public_key).into())
                    } else {
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .text("Select")
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
                        .push(select_btn.width(Length::Fixed(180.0)).view())
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
                            Button::new()
                                .text("Share")
                                .width(Length::Fixed(180.0))
                                .on_press(ShareSignerMessage::Share.into())
                                .view(),
                        )
                        .width(Length::Fill)
                        .align_items(Alignment::End),
                );
            }
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
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
