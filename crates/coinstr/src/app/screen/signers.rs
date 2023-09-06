// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::types::{GetSharedSigner, GetSigner};
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::icon::{CLIPBOARD, FULLSCREEN, PLUS, RELOAD, SHARE, TRASH};

#[derive(Debug, Clone)]
pub enum SignersMessage {
    LoadSigners((Vec<GetSigner>, Vec<GetSharedSigner>)),
    Reload,
}

#[derive(Debug, Default)]
pub struct SignersState {
    loading: bool,
    loaded: bool,
    signers: Vec<GetSigner>,
    shared_signers: Vec<GetSharedSigner>,
}

impl SignersState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for SignersState {
    fn title(&self) -> String {
        String::from("Signers")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let signers = client.get_signers().await.unwrap();
                let shared_signers = client.get_shared_signers().await.unwrap();
                (signers, shared_signers)
            },
            |signers| SignersMessage::LoadSigners(signers).into(),
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Signers(msg) = message {
            match msg {
                SignersMessage::LoadSigners((signers, shared_signers)) => {
                    self.signers = signers;
                    self.shared_signers = shared_signers;
                    self.loading = false;
                    self.loaded = true;
                    Command::none()
                }
                SignersMessage::Reload => self.load(ctx),
            }
        } else {
            Command::none()
        }
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;

        if self.loaded {
            if self.signers.is_empty() && self.shared_signers.is_empty() {
                let add_signer_btn = Button::new()
                    .icon(PLUS)
                    .text("Add signer")
                    .width(Length::Fixed(250.0))
                    .on_press(Message::View(Stage::AddSigner))
                    .view();
                let reload_btn = Button::new()
                    .style(ButtonStyle::Bordered)
                    .icon(RELOAD)
                    .text("Reload")
                    .width(Length::Fixed(250.0))
                    .on_press(SignersMessage::Reload.into())
                    .view();
                content = content
                    .push(Text::new("No signers").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(add_signer_btn)
                    .push(reload_btn)
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                let add_signer_btn = Button::new()
                    .style(ButtonStyle::Bordered)
                    .icon(PLUS)
                    .width(Length::Fixed(40.0))
                    .on_press(Message::View(Stage::AddSigner))
                    .view();
                let revoke_all_btn = Button::new()
                    .style(ButtonStyle::BorderedDanger)
                    .icon(TRASH)
                    .width(Length::Fixed(40.0))
                    .on_press(Message::View(Stage::RevokeAllSigners))
                    .view();
                let reload_btn = Button::new()
                    .style(ButtonStyle::Bordered)
                    .icon(RELOAD)
                    .width(Length::Fixed(40.0))
                    .on_press(SignersMessage::Reload.into())
                    .loading(self.loading)
                    .view();

                // My Signers

                content = content
                    .push(Text::new("My Signers").big().bold().view())
                    .push(
                        Row::new()
                            .push(
                                Text::new("ID")
                                    .bold()
                                    .big()
                                    .width(Length::Fixed(115.0))
                                    .view(),
                            )
                            .push(Text::new("Name").bold().big().width(Length::Fill).view())
                            .push(
                                Text::new("Fingerprint")
                                    .bold()
                                    .big()
                                    .width(Length::Fixed(175.0))
                                    .view(),
                            )
                            .push(
                                Text::new("Type")
                                    .bold()
                                    .big()
                                    .width(Length::Fixed(125.0))
                                    .view(),
                            )
                            .push(add_signer_btn)
                            .push(revoke_all_btn)
                            .push(reload_btn)
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for GetSigner { signer_id, signer } in self.signers.iter() {
                    let row = Row::new()
                        .push(
                            Text::new(util::cut_event_id(*signer_id))
                                .width(Length::Fixed(115.0))
                                .view(),
                        )
                        .push(Text::new(signer.name()).width(Length::Fill).view())
                        .push(
                            Text::new(signer.fingerprint().to_string())
                                .width(Length::Fixed(175.0))
                                .view(),
                        )
                        .push(
                            Text::new(signer.signer_type().to_string())
                                .width(Length::Fixed(125.0))
                                .view(),
                        )
                        .push(
                            Button::new()
                                .style(ButtonStyle::Bordered)
                                .icon(CLIPBOARD)
                                .on_press(Message::Clipboard(
                                    signer
                                        .descriptor_public_key()
                                        .map(|d| d.to_string())
                                        .unwrap_or_default(),
                                ))
                                .width(Length::Fixed(40.0))
                                .view(),
                        )
                        .push(
                            Button::new()
                                .style(ButtonStyle::Bordered)
                                .icon(SHARE)
                                .width(Length::Fixed(40.0))
                                .on_press(Message::View(Stage::ShareSigner(*signer_id)))
                                .view(),
                        )
                        .push(
                            Button::new()
                                .icon(FULLSCREEN)
                                .width(Length::Fixed(40.0))
                                .on_press(Message::View(Stage::Signer(*signer_id, signer.clone())))
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill);
                    content = content.push(row).push(rule::horizontal());
                }

                // Shared Signers

                if !self.shared_signers.is_empty() {
                    content = content
                        .push(Space::with_height(Length::Fixed(40.0)))
                        .push(Text::new("Contacts's Signers").big().bold().view())
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
                                    Text::new("Fingerprint")
                                        .bold()
                                        .big()
                                        .width(Length::Fixed(175.0))
                                        .view(),
                                )
                                .push(Text::new("Owner").bold().big().width(Length::Fill).view())
                                .push(Space::with_width(Length::Fixed(40.0)))
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill),
                        )
                        .push(rule::horizontal_bold());

                    for GetSharedSigner {
                        shared_signer_id,
                        owner_public_key: _,
                        shared_signer,
                    } in self.shared_signers.iter()
                    {
                        let row = Row::new()
                            .push(
                                Text::new(util::cut_event_id(*shared_signer_id))
                                    .width(Length::Fixed(115.0))
                                    .view(),
                            )
                            .push(
                                Text::new(shared_signer.fingerprint().to_string())
                                    .width(Length::Fixed(175.0))
                                    .view(),
                            )
                            .push(
                                // TODO
                                Text::new("TODO").width(Length::Fill).view(), /* Text::new(ctx.client.db.get_public_key_name(*owner_public_key))
                                                                              .width(Length::Fill)
                                                                              .view() */
                            )
                            .push(
                                Button::new()
                                    .style(ButtonStyle::Bordered)
                                    .icon(CLIPBOARD)
                                    .on_press(Message::Clipboard(
                                        shared_signer
                                            .descriptor_public_key()
                                            .map(|d| d.to_string())
                                            .unwrap_or_default(),
                                    ))
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
        }

        Dashboard::new()
            .loaded(self.loaded)
            .view(ctx, content, true, center_y)
    }
}

impl From<SignersState> for Box<dyn State> {
    fn from(s: SignersState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<SignersMessage> for Message {
    fn from(msg: SignersMessage) -> Self {
        Self::Signers(msg)
    }
}
