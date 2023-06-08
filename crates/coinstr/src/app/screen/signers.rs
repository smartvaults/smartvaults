// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_sdk::core::signer::Signer;
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::constants::APP_NAME;
use crate::theme::icon::{FULLSCREEN, PLUS, RELOAD, SHARE};

#[derive(Debug, Clone)]
pub enum SignersMessage {
    LoadSigners(BTreeMap<EventId, Signer>),
    Reload,
}

#[derive(Debug, Default)]
pub struct SignersState {
    loading: bool,
    loaded: bool,
    signers: BTreeMap<EventId, Signer>,
}

impl SignersState {
    pub fn new() -> Self {
        Self::default()
    }
}

impl State for SignersState {
    fn title(&self) -> String {
        format!("{APP_NAME} - Signers")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(async move { client.get_signers().unwrap() }, |signers| {
            SignersMessage::LoadSigners(signers).into()
        })
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Signers(msg) = message {
            match msg {
                SignersMessage::LoadSigners(signers) => {
                    self.signers = signers;
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
            if self.signers.is_empty() {
                let add_policy_btn = button::primary_with_icon(PLUS, "Add signer")
                    .width(Length::Fixed(250.0))
                    .on_press(Message::View(Stage::AddSigner));
                let reload_btn = button::border_with_icon(RELOAD, "Reload")
                    .width(Length::Fixed(250.0))
                    .on_press(SignersMessage::Reload.into());
                content = content
                    .push(Text::new("No signers").view())
                    .push(Space::with_height(Length::Fixed(15.0)))
                    .push(add_policy_btn)
                    .push(reload_btn)
                    .align_items(Alignment::Center);
            } else {
                center_y = false;

                let add_policy_btn = button::border_only_icon(PLUS)
                    .width(Length::Fixed(40.0))
                    .on_press(Message::View(Stage::AddPolicy));
                let mut reload_btn = button::border_only_icon(RELOAD).width(Length::Fixed(40.0));

                if !self.loading {
                    reload_btn = reload_btn.on_press(SignersMessage::Reload.into());
                }

                content = content
                    .push(
                        Row::new()
                            .push(
                                Text::new("ID")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fixed(115.0))
                                    .view(),
                            )
                            .push(Text::new("Name").bold().bigger().width(Length::Fill).view())
                            .push(
                                Text::new("Fingerprint")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fixed(175.0))
                                    .view(),
                            )
                            .push(
                                Text::new("Type")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fixed(125.0))
                                    .view(),
                            )
                            .push(add_policy_btn)
                            .push(reload_btn)
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (signer_id, signer) in self.signers.iter() {
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
                        .push(button::border_only_icon(SHARE).width(Length::Fixed(40.0)))
                        .push(button::primary_only_icon(FULLSCREEN).width(Length::Fixed(40.0)))
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
