// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::Signer;
use smartvaults_sdk::types::{GetSharedSigner, GetSigner, GetSignerOffering};
use smartvaults_sdk::util;

use crate::app::component::Dashboard;
use crate::app::context::Mode;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text};
use crate::theme::icon::{CLIPBOARD, FULLSCREEN, PENCIL, PLUS, RELOAD, SHARE, TRASH};

#[derive(Debug, Clone)]
pub enum SignersMessage {
    Load {
        signers: Vec<GetSigner>,
        shared_signers: Vec<GetSharedSigner>,
        signer_offerings: Vec<GetSignerOffering>,
    },
    DeleteSignerOffering(Signer),
    Reload,
}

#[derive(Debug, Default)]
pub struct SignersState {
    loading: bool,
    loaded: bool,
    signers: Vec<GetSigner>,
    shared_signers: Vec<GetSharedSigner>,
    signer_offerings: Vec<GetSignerOffering>,
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
        let mode = ctx.mode;
        Command::perform(
            async move {
                let signers = client.get_signers().await;
                let shared_signers = match mode {
                    Mode::User => client.get_shared_signers().await.unwrap(),
                    Mode::KeyAgent => Vec::new(),
                };
                let signer_offerings = match mode {
                    Mode::User => Vec::new(),
                    Mode::KeyAgent => client.my_signer_offerings().await.unwrap(),
                };
                (signers, shared_signers, signer_offerings)
            },
            |(signers, shared_signers, signer_offerings)| {
                SignersMessage::Load {
                    signers,
                    shared_signers,
                    signer_offerings,
                }
                .into()
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Signers(msg) = message {
            match msg {
                SignersMessage::Load {
                    signers,
                    shared_signers,
                    signer_offerings,
                } => {
                    self.signers = signers;
                    self.shared_signers = shared_signers;
                    self.signer_offerings = signer_offerings;
                    self.loading = false;
                    self.loaded = true;
                    Command::none()
                }
                SignersMessage::DeleteSignerOffering(signer) => {
                    let client = ctx.client.clone();
                    self.loading = true;
                    Command::perform(
                        async move { client.delete_signer_offering(&signer).await },
                        |res| match res {
                            Ok(_) => SignersMessage::Reload.into(),
                            Err(e) => {
                                tracing::error!("Impossible to delete signer offering: {e}");
                                SignersMessage::Reload.into()
                            }
                        },
                    )
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
                    .loading(self.loading || ctx.mode.is_key_agent())
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
                if !self.shared_signers.is_empty() && !ctx.mode.is_key_agent() {
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
                        owner,
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
                            .push(Text::new(owner.name()).width(Length::Fill).view())
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

                // Signer offerings
                if ctx.mode.is_key_agent() {
                    content = content
                        .push(Space::with_height(Length::Fixed(40.0)))
                        .push(
                            Row::new()
                                .push(
                                    Text::new("Signer offerings")
                                        .bold()
                                        .big()
                                        .width(Length::Fill)
                                        .view(),
                                )
                                .push(
                                    Button::new()
                                        .style(ButtonStyle::Bordered)
                                        .icon(PLUS)
                                        .width(Length::Fixed(40.0))
                                        .on_press(Message::View(Stage::EditSignerOffering(None)))
                                        .view(),
                                )
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill),
                        )
                        .push(rule::horizontal_bold());

                    for GetSignerOffering {
                        id: _,
                        signer,
                        offering,
                    } in self.signer_offerings.iter()
                    {
                        let row = Row::new()
                            .push(
                                Column::new()
                                    .push(Text::new(format!("Name: {}", signer.name())).view())
                                    .push(
                                        Text::new(format!("Fingerprint: {}", signer.fingerprint()))
                                            .view(),
                                    )
                                    .push(
                                        Text::new(format!("Type: {}", signer.signer_type())).view(),
                                    )
                                    .spacing(10)
                                    .width(Length::Fill),
                            )
                            .push(
                                Column::new()
                                    .push(
                                        Text::new(format!("Temperature: {}", offering.temperature))
                                            .view(),
                                    )
                                    .push(
                                        Text::new(format!(
                                            "Response time: {} min",
                                            offering.response_time
                                        ))
                                        .view(),
                                    )
                                    .push(
                                        Text::new(format!("Device type: {}", offering.device_type))
                                            .view(),
                                    )
                                    .push(
                                        Text::new(format!(
                                            "Yearly cost (basis points): {}",
                                            offering.yearly_cost_basis_points.unwrap_or_default()
                                        ))
                                        .view(),
                                    )
                                    .push(
                                        Text::new(format!(
                                            "Yearly cost: {}",
                                            offering
                                                .yearly_cost
                                                .as_ref()
                                                .map(|p| p.to_string())
                                                .unwrap_or_else(|| String::from("None"))
                                        ))
                                        .view(),
                                    )
                                    .push(
                                        Text::new(format!(
                                            "Cost per signature: {}",
                                            offering
                                                .cost_per_signature
                                                .as_ref()
                                                .map(|p| p.to_string())
                                                .unwrap_or_else(|| String::from("None"))
                                        ))
                                        .view(),
                                    )
                                    .spacing(10)
                                    .width(Length::Fill),
                            )
                            .push(
                                Button::new()
                                    .style(ButtonStyle::Bordered)
                                    .icon(PENCIL)
                                    .width(Length::Fixed(40.0))
                                    .on_press(Message::View(Stage::EditSignerOffering(Some((
                                        signer.clone(),
                                        Some(*offering),
                                    )))))
                                    .view(),
                            )
                            .push(
                                Button::new()
                                    .style(ButtonStyle::BorderedDanger)
                                    .icon(TRASH)
                                    .width(Length::Fixed(40.0))
                                    .on_press(
                                        SignersMessage::DeleteSignerOffering(signer.signer.clone())
                                            .into(),
                                    )
                                    .loading(self.loading)
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
