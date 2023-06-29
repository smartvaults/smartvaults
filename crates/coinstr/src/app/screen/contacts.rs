// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_sdk::nostr::bitcoin::XOnlyPublicKey;
use coinstr_sdk::nostr::Metadata;
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::constants::APP_NAME;
use crate::theme::icon::{PLUS, RELOAD, TRASH};

#[derive(Debug, Clone)]
pub enum ContactsMessage {
    LoadContacts(BTreeMap<XOnlyPublicKey, Metadata>),
    RemovePublicKey(XOnlyPublicKey),
    ErrorChanged(Option<String>),
    Reload,
}

#[derive(Debug, Default)]
pub struct ContactsState {
    loading: bool,
    loaded: bool,
    contacts: BTreeMap<XOnlyPublicKey, Metadata>,
    error: Option<String>,
}

impl ContactsState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for ContactsState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Contacts")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(async move { client.get_contacts().unwrap() }, |p| {
            ContactsMessage::LoadContacts(p).into()
        })
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Contacts(msg) = message {
            match msg {
                ContactsMessage::LoadContacts(contacts) => {
                    self.contacts = contacts;
                    self.loading = false;
                    self.loaded = true;
                }
                ContactsMessage::RemovePublicKey(public_key) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.remove_contact(public_key).await },
                        |res| match res {
                            Ok(_) => ContactsMessage::Reload.into(),
                            Err(e) => ContactsMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ContactsMessage::ErrorChanged(error) => {
                    self.error = error;
                    self.loading = false;
                }
                ContactsMessage::Reload => return self.load(ctx),
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
                    .on_press(ContactsMessage::Reload.into());
                content = content
                    .push(Text::new("No contacts").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(add_contact_btn)
                    .push(reload_btn)
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                let add_contact_btn = button::border_only_icon(PLUS)
                    .width(Length::Fixed(40.0))
                    .on_press(Message::View(Stage::AddContact));
                let mut reload_btn = button::border_only_icon(RELOAD).width(Length::Fixed(40.0));

                if !self.loading {
                    reload_btn = reload_btn.on_press(ContactsMessage::Reload.into());
                }

                content = content
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
                            .push(
                                Text::new("NIP-05")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(add_contact_btn)
                            .push(reload_btn)
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (public_key, metadata) in self.contacts.iter() {
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
                        .push(
                            Text::new(metadata.nip05.as_deref().unwrap_or_default())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(Space::with_width(Length::Fixed(40.0)))
                        .push(
                            button::danger_border_only_icon(TRASH)
                                .width(Length::Fixed(40.0))
                                .on_press(ContactsMessage::RemovePublicKey(*public_key).into()),
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

impl From<ContactsState> for Box<dyn State> {
    fn from(s: ContactsState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ContactsMessage> for Message {
    fn from(msg: ContactsMessage) -> Self {
        Self::Contacts(msg)
    }
}
