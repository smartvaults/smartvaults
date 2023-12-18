// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::signer::Signer;
use smartvaults_sdk::nostr::EventId;
use smartvaults_sdk::nostr::Profile;
use smartvaults_sdk::util;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::color::RED;
use crate::theme::icon::TRASH;

#[derive(Debug, Clone)]
pub enum SignerMessage {
    LoadMySharedSigners(BTreeMap<EventId, Profile>),
    Delete,
    RevokeSharedSigner(EventId),
    Reload,
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct SignerState {
    loading: bool,
    loaded: bool,
    signer_id: EventId,
    signer: Signer,
    my_shared_signers: BTreeMap<EventId, Profile>,
    error: Option<String>,
}

impl SignerState {
    pub fn new(signer_id: EventId, signer: Signer) -> Self {
        Self {
            loading: false,
            loaded: false,
            signer_id,
            signer,
            my_shared_signers: BTreeMap::new(),
            error: None,
        }
    }
}

impl State for SignerState {
    fn title(&self) -> String {
        format!("Signer #{}", util::cut_event_id(self.signer_id))
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        let signer_id = self.signer_id;
        Command::perform(
            async move {
                client
                    .get_my_shared_signers_by_signer_id(signer_id)
                    .await
                    .unwrap()
            },
            |signers| SignerMessage::LoadMySharedSigners(signers).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Signer(msg) = message {
            match msg {
                SignerMessage::LoadMySharedSigners(signers) => {
                    self.my_shared_signers = signers;
                    self.loading = false;
                    self.loaded = true;
                }
                SignerMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                SignerMessage::Delete => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let signer_id = self.signer_id;
                    return Command::perform(
                        async move { client.delete_signer_by_id(signer_id).await },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Signers),
                            Err(e) => SignerMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                SignerMessage::RevokeSharedSigner(shared_signer_id) => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    return Command::perform(
                        async move { client.revoke_shared_signer(shared_signer_id).await },
                        |res| match res {
                            Ok(_) => SignerMessage::Reload.into(),
                            Err(e) => SignerMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                SignerMessage::Reload => return self.load(ctx),
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);

        if self.loaded {
            content = content
                .push(
                    Text::new(format!("Signer #{}", util::cut_event_id(self.signer_id)))
                        .size(40)
                        .bold()
                        .view(),
                )
                .push(Space::with_height(Length::Fixed(40.0)))
                .push(Text::new(format!("Name: {}", self.signer.name())).view())
                .push(Text::new(format!("Type: {}", self.signer.signer_type())).view())
                .push(Text::new(format!("Fingerprint: {}", self.signer.fingerprint())).view())
                .push(Text::new(format!("Descriptor: {}", self.signer.descriptor())).view())
                .push(Space::with_height(10.0))
                .push(
                    Row::new()
                        .push(
                            Button::new()
                                .style(ButtonStyle::Danger)
                                .icon(TRASH)
                                .text("Delete")
                                .on_press(SignerMessage::Delete.into())
                                .loading(self.loading)
                                .view(),
                        )
                        .spacing(10),
                )
                .push(Space::with_height(20.0));

            if let Some(error) = &self.error {
                content = content.push(Text::new(error).color(RED).view());
            };

            if !self.my_shared_signers.is_empty() {
                content = content
                    .push(Text::new("My Shared Signers").bold().big().view())
                    .push(Space::with_height(10.0))
                    .push(
                        Row::new()
                            .push(
                                Text::new("ID")
                                    .bold()
                                    .big()
                                    .width(Length::Fixed(115.0))
                                    .view(),
                            )
                            .push(Text::new("User").bold().big().width(Length::Fill).view())
                            .push(Space::with_width(Length::Fixed(40.0)))
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (shared_signer_id, user) in self.my_shared_signers.iter() {
                    let row = Row::new()
                        .push(
                            Text::new(util::cut_event_id(*shared_signer_id))
                                .width(Length::Fixed(115.0))
                                .view(),
                        )
                        .push(Text::new(user.name()).width(Length::Fill).view())
                        .push(
                            Button::new()
                                .style(ButtonStyle::BorderedDanger)
                                .icon(TRASH)
                                .on_press(
                                    SignerMessage::RevokeSharedSigner(*shared_signer_id).into(),
                                )
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

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, false, false)
    }
}

impl From<SignerState> for Box<dyn State> {
    fn from(s: SignerState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SignerMessage> for Message {
    fn from(msg: SignerMessage) -> Self {
        Self::Signer(msg)
    }
}
