// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bdk::miniscript::DescriptorPublicKey;
use coinstr_sdk::core::bitcoin::XOnlyPublicKey;
use coinstr_sdk::db::model::{GetAllSigners, GetSharedSignerResult};
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text, TextInput};
use crate::theme::color::DARK_RED;
use crate::theme::icon::TRASH;

#[derive(Debug, Clone)]
pub enum PolicyBuilderMessage {
    NameChanged(String),
    DescriptionChanged(String),
    IncreaseThreshold,
    DecreaseThreshold,
    LoadAllSigners(GetAllSigners),
    AddSigner,
    EditSigner(usize, XOnlyPublicKey, Box<DescriptorPublicKey>),
    RemoveSigner(usize),
    SelectingSigner { index: Option<usize> },
    ErrorChanged(Option<String>),
    SavePolicy,
}

#[derive(Debug, Default)]
pub struct PolicyBuilderState {
    name: String,
    description: String,
    signers: GetAllSigners,
    threshold: usize,
    policy: Vec<Option<(XOnlyPublicKey, DescriptorPublicKey)>>,
    loading: bool,
    loaded: bool,
    selecting_signer: Option<usize>,
    error: Option<String>,
}

impl PolicyBuilderState {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_already_selected(&self, descriptor: &DescriptorPublicKey) -> bool {
        for (_, desc) in self.policy.iter().flatten() {
            if desc == descriptor {
                return true;
            }
        }

        false
    }

    fn pk_is_already_selected(&self, public_key: XOnlyPublicKey) -> bool {
        for (pk, ..) in self.policy.iter().flatten() {
            if pk == &public_key {
                return true;
            }
        }

        false
    }
}

impl State for PolicyBuilderState {
    fn title(&self) -> String {
        String::from("Policy builder")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(async move { client.get_all_signers().unwrap() }, |s| {
            PolicyBuilderMessage::LoadAllSigners(s).into()
        })
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::PolicyBuilder(msg) = message {
            match msg {
                PolicyBuilderMessage::NameChanged(name) => self.name = name,
                PolicyBuilderMessage::DescriptionChanged(desc) => self.description = desc,
                PolicyBuilderMessage::IncreaseThreshold => {
                    if self.threshold < self.policy.len() {
                        self.threshold += 1;
                    }
                }
                PolicyBuilderMessage::DecreaseThreshold => {
                    let new_threshold = self.threshold.saturating_sub(1);
                    if new_threshold >= 1 {
                        self.threshold = new_threshold;
                    }
                }
                PolicyBuilderMessage::ErrorChanged(error) => self.error = error,
                PolicyBuilderMessage::LoadAllSigners(signers) => {
                    self.signers = signers;
                    self.loading = false;
                    self.loaded = true;
                }
                PolicyBuilderMessage::AddSigner => {
                    self.policy.push(None);
                    if self.threshold == 0 {
                        self.threshold = self.policy.len();
                    }
                }
                PolicyBuilderMessage::EditSigner(index, pk, desc) => {
                    self.selecting_signer = None;
                    match self.policy.get_mut(index) {
                        Some(v) => *v = Some((pk, *desc)),
                        None => {
                            self.error =
                                Some(String::from("Impossible to edit signer: index not found"))
                        }
                    };
                }
                PolicyBuilderMessage::RemoveSigner(index) => {
                    self.policy.remove(index);
                    let len = self.policy.len();
                    if self.threshold > len {
                        self.threshold = len;
                    }
                }
                PolicyBuilderMessage::SelectingSigner { index } => self.selecting_signer = index,
                PolicyBuilderMessage::SavePolicy => {
                    let client = ctx.client.clone();
                    let name = self.name.clone();
                    let description = self.description.clone();
                    let threshold = self.threshold;
                    let descriptors: Vec<DescriptorPublicKey> = self
                        .policy
                        .iter()
                        .flatten()
                        .map(|(_, desc)| desc.clone())
                        .collect();
                    let public_keys: Vec<XOnlyPublicKey> =
                        self.policy.iter().flatten().map(|(pk, ..)| *pk).collect();
                    return Command::perform(
                        async move {
                            let custom_pubkeys = if public_keys.is_empty() {
                                None
                            } else {
                                Some(public_keys)
                            };
                            let descriptor =
                                coinstr_sdk::core::policy::builder::n_of_m_ext_multisig(
                                    threshold,
                                    descriptors,
                                )?;
                            client
                                .save_policy(name, description, descriptor, custom_pubkeys)
                                .await?;
                            Ok::<(), Box<dyn std::error::Error>>(())
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Policies),
                            Err(e) => {
                                PolicyBuilderMessage::ErrorChanged(Some(e.to_string())).into()
                            }
                        },
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut center_y = true;
        let content = if let Some(index) = self.selecting_signer {
            center_y = false;

            view_signer_selector(self, ctx, index)
        } else {
            let name = TextInput::new("Name", &self.name)
                .on_input(|s| PolicyBuilderMessage::NameChanged(s).into())
                .placeholder("Policy name")
                .view();

            let description = TextInput::new("Description", &self.description)
                .on_input(|s| PolicyBuilderMessage::DescriptionChanged(s).into())
                .placeholder("Policy description")
                .view();

            let threshold = Row::new()
                .push(
                    Text::new(format!(
                        "Threshold: {}/{}",
                        self.threshold,
                        self.policy.len()
                    ))
                    .view(),
                )
                .push(
                    button::border("+")
                        .on_press(PolicyBuilderMessage::IncreaseThreshold.into())
                        .width(Length::Fixed(40.0)),
                )
                .push(
                    button::border("-")
                        .on_press(PolicyBuilderMessage::DecreaseThreshold.into())
                        .width(Length::Fixed(40.0)),
                )
                .spacing(10)
                .align_items(Alignment::Center);

            let mut pks = Column::new().spacing(10);

            for (index, value) in self.policy.iter().enumerate() {
                match value {
                    Some((pk, desc)) => {
                        pks = pks.push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .push(
                                            Text::new(format!(
                                                "User: {}",
                                                ctx.client.db.get_public_key_name(*pk)
                                            ))
                                            .smaller()
                                            .extra_light()
                                            .view(),
                                        )
                                        .push(
                                            Text::new(format!(
                                                "Fingerprint: {}",
                                                desc.master_fingerprint()
                                            ))
                                            .smaller()
                                            .extra_light()
                                            .view(),
                                        )
                                        .spacing(5)
                                        .width(Length::Fill),
                                )
                                .push(
                                    button::danger_border_only_icon(TRASH)
                                        .on_press(PolicyBuilderMessage::RemoveSigner(index).into())
                                        .width(Length::Fixed(40.0)),
                                )
                                .spacing(10)
                                .align_items(Alignment::Center)
                                .width(Length::Fill),
                        )
                    }
                    None => {
                        pks = pks.push(
                            Row::new()
                                .push(
                                    button::primary("Set").width(Length::Fill).on_press(
                                        PolicyBuilderMessage::SelectingSigner {
                                            index: Some(index),
                                        }
                                        .into(),
                                    ),
                                )
                                .push(
                                    button::danger_border_only_icon(TRASH)
                                        .on_press(PolicyBuilderMessage::RemoveSigner(index).into())
                                        .width(Length::Fixed(40.0)),
                                )
                                .spacing(10),
                        )
                    }
                }
            }

            let add_new_pk_btn = button::border("Add new slot")
                .on_press(PolicyBuilderMessage::AddSigner.into())
                .width(Length::Fill);

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            let restore_policy_btn = button::border("Restore policy backup")
                .on_press(Message::View(Stage::RestorePolicy))
                .width(Length::Fill);

            let save_policy_btn = button::primary("Save policy")
                .on_press(PolicyBuilderMessage::SavePolicy.into())
                .width(Length::Fill);

            Column::new()
                .push(
                    Column::new()
                        .push(Text::new("Policy builder").size(24).bold().view())
                        .push(Text::new("Build a new policy").extra_light().view())
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(name)
                .push(description)
                .push(Space::with_height(Length::Fixed(5.0)))
                .push(threshold)
                .push(pks)
                .push(add_new_pk_btn)
                .push(error)
                .push(Space::with_height(Length::Fixed(15.0)))
                .push(save_policy_btn)
                .push(restore_policy_btn)
                .align_items(Alignment::Center)
                .spacing(10)
                .padding(20)
                .max_width(400)
        };

        Dashboard::new().view(ctx, content, true, center_y)
    }
}

fn view_signer_selector<'a>(
    state: &PolicyBuilderState,
    ctx: &Context,
    index: usize,
) -> Column<'a, Message> {
    let mut content = Column::new().spacing(10).padding(20);

    // My Signers

    content = content
        .push(Text::new("My Signers").bigger().bold().view())
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
                .push(Space::with_width(Length::Fixed(180.0)))
                .spacing(10)
                .align_items(Alignment::Center)
                .width(Length::Fill),
        )
        .push(rule::horizontal_bold());

    let public_key = ctx.client.keys().public_key();
    for (signer_id, signer) in state.signers.my.iter() {
        if let Ok(descriptor) = signer.descriptor_public_key() {
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
                .push(if state.is_already_selected(&descriptor) {
                    button::primary("Selected").width(Length::Fixed(180.0))
                } else if state.pk_is_already_selected(public_key) {
                    button::border("Select").width(Length::Fixed(180.0))
                } else {
                    button::border("Select")
                        .width(Length::Fixed(180.0))
                        .on_press(
                            PolicyBuilderMessage::EditSigner(
                                index,
                                public_key,
                                Box::new(descriptor),
                            )
                            .into(),
                        )
                })
                .spacing(10)
                .align_items(Alignment::Center)
                .width(Length::Fill);
            content = content.push(row).push(rule::horizontal());
        }
    }

    // Shared Signers

    content = content
        .push(Space::with_height(Length::Fixed(40.0)))
        .push(Text::new("Contacts's Signers").bigger().bold().view())
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
                    Text::new("Fingerprint")
                        .bold()
                        .bigger()
                        .width(Length::Fixed(175.0))
                        .view(),
                )
                .push(
                    Text::new("Owner")
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

    for (
        shared_signer_id,
        GetSharedSignerResult {
            owner_public_key,
            shared_signer,
        },
    ) in state.signers.contacts.iter()
    {
        if let Ok(descriptor) = shared_signer.descriptor_public_key() {
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
                    Text::new(ctx.client.db.get_public_key_name(*owner_public_key))
                        .width(Length::Fill)
                        .view(),
                )
                .push(if state.is_already_selected(&descriptor) {
                    button::primary("Selected").width(Length::Fixed(180.0))
                } else if state.pk_is_already_selected(*owner_public_key) {
                    button::border("Select").width(Length::Fixed(180.0))
                } else {
                    button::border("Select")
                        .width(Length::Fixed(180.0))
                        .on_press(
                            PolicyBuilderMessage::EditSigner(
                                index,
                                *owner_public_key,
                                Box::new(descriptor),
                            )
                            .into(),
                        )
                })
                .spacing(10)
                .align_items(Alignment::Center)
                .width(Length::Fill);
            content = content.push(row).push(rule::horizontal());
        }
    }

    content
}

impl From<PolicyBuilderState> for Box<dyn State> {
    fn from(s: PolicyBuilderState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<PolicyBuilderMessage> for Message {
    fn from(msg: PolicyBuilderMessage) -> Self {
        Self::PolicyBuilder(msg)
    }
}
