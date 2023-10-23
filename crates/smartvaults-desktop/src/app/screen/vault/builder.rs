// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use smartvaults_sdk::core::miniscript::DescriptorPublicKey;
use smartvaults_sdk::core::secp256k1::XOnlyPublicKey;
use smartvaults_sdk::core::PolicyTemplate;
use smartvaults_sdk::types::{GetAllSigners, GetSharedSigner, GetSigner, User};
use smartvaults_sdk::util;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Button, ButtonStyle, Text, TextInput};
use crate::theme::color::DARK_RED;
use crate::theme::icon::TRASH;

#[derive(Debug, Clone)]
pub enum PolicyBuilderMessage {
    NameChanged(String),
    DescriptionChanged(String),
    IncreaseThreshold,
    DecreaseThreshold,
    Load((GetAllSigners, User)),
    AddSigner,
    EditSigner(usize, Box<User>, Box<DescriptorPublicKey>),
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
    policy: Vec<Option<(User, DescriptorPublicKey)>>,
    profile: Option<User>,
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
        for (user, ..) in self.policy.iter().flatten() {
            if user.public_key() == public_key {
                return true;
            }
        }

        false
    }
}

impl State for PolicyBuilderState {
    fn title(&self) -> String {
        String::from("Vault builder")
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        if self.loading {
            return Command::none();
        }

        self.loading = true;
        let client = ctx.client.clone();
        Command::perform(
            async move {
                let signers = client.get_all_signers().await.unwrap();
                let profile = client.get_profile().await.unwrap();
                (signers, profile)
            },
            |(s, p)| PolicyBuilderMessage::Load((s, p)).into(),
        )
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
                PolicyBuilderMessage::Load((signers, profile)) => {
                    self.signers = signers;
                    self.profile = Some(profile);
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
                        Some(v) => *v = Some((*pk, *desc)),
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
                    let public_keys: Vec<XOnlyPublicKey> = self
                        .policy
                        .iter()
                        .flatten()
                        .map(|(user, ..)| user.public_key())
                        .collect();
                    return Command::perform(
                        async move {
                            let template: PolicyTemplate =
                                PolicyTemplate::multisig(threshold, descriptors);
                            let policy: String = template.build()?.to_string();
                            client
                                .save_policy(name, description, policy, public_keys)
                                .await?;
                            Ok::<(), Box<dyn std::error::Error>>(())
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Vaults),
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

            view_signer_selector(self, index)
        } else {
            let name = TextInput::new("Name", &self.name)
                .on_input(|s| PolicyBuilderMessage::NameChanged(s).into())
                .placeholder("Vault name")
                .view();

            let description = TextInput::new("Description", &self.description)
                .on_input(|s| PolicyBuilderMessage::DescriptionChanged(s).into())
                .placeholder("Vault description")
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
                    Button::new()
                        .style(ButtonStyle::Bordered)
                        .text("+")
                        .on_press(PolicyBuilderMessage::IncreaseThreshold.into())
                        .width(Length::Fixed(40.0))
                        .view(),
                )
                .push(
                    Button::new()
                        .style(ButtonStyle::Bordered)
                        .text("-")
                        .on_press(PolicyBuilderMessage::DecreaseThreshold.into())
                        .width(Length::Fixed(40.0))
                        .view(),
                )
                .spacing(10)
                .align_items(Alignment::Center);

            let mut pks = Column::new().spacing(10);

            for (index, value) in self.policy.iter().enumerate() {
                match value {
                    Some((user, desc)) => {
                        pks = pks.push(
                            Row::new()
                                .push(
                                    Column::new()
                                        .push(
                                            Text::new(format!("User: {}", user.name()))
                                                .small()
                                                .extra_light()
                                                .view(),
                                        )
                                        .push(
                                            Text::new(format!(
                                                "Fingerprint: {}",
                                                desc.master_fingerprint()
                                            ))
                                            .small()
                                            .extra_light()
                                            .view(),
                                        )
                                        .spacing(5)
                                        .width(Length::Fill),
                                )
                                .push(
                                    Button::new()
                                        .style(ButtonStyle::BorderedDanger)
                                        .icon(TRASH)
                                        .on_press(PolicyBuilderMessage::RemoveSigner(index).into())
                                        .width(Length::Fixed(40.0))
                                        .view(),
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
                                    Button::new()
                                        .text("Set")
                                        .width(Length::Fill)
                                        .on_press(
                                            PolicyBuilderMessage::SelectingSigner {
                                                index: Some(index),
                                            }
                                            .into(),
                                        )
                                        .view(),
                                )
                                .push(
                                    Button::new()
                                        .style(ButtonStyle::BorderedDanger)
                                        .icon(TRASH)
                                        .on_press(PolicyBuilderMessage::RemoveSigner(index).into())
                                        .width(Length::Fixed(40.0))
                                        .view(),
                                )
                                .spacing(10),
                        )
                    }
                }
            }

            let add_new_pk_btn = Button::new()
                .style(ButtonStyle::Bordered)
                .text("Add new slot")
                .on_press(PolicyBuilderMessage::AddSigner.into())
                .width(Length::Fill)
                .view();

            let error = if let Some(error) = &self.error {
                Row::new().push(Text::new(error).color(DARK_RED).view())
            } else {
                Row::new()
            };

            let restore_policy_btn = Button::new()
                .style(ButtonStyle::Bordered)
                .text("Restore vault backup")
                .on_press(Message::View(Stage::RestoreVault))
                .width(Length::Fill)
                .view();

            let save_policy_btn = Button::new()
                .text("Save vault")
                .on_press(PolicyBuilderMessage::SavePolicy.into())
                .width(Length::Fill)
                .view();

            Column::new()
                .push(
                    Column::new()
                        .push(Text::new("Vault builder").big().bold().view())
                        .push(Text::new("Build a new vault").extra_light().view())
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

fn view_signer_selector<'a>(state: &PolicyBuilderState, index: usize) -> Column<'a, Message> {
    let mut content = Column::new().spacing(10).padding(20);

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
                .push(Space::with_width(Length::Fixed(180.0)))
                .spacing(10)
                .align_items(Alignment::Center)
                .width(Length::Fill),
        )
        .push(rule::horizontal_bold());

    if let Some(user) = &state.profile {
        for GetSigner { signer_id, signer } in state.signers.my.iter() {
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
                        Button::new()
                            .text("Selected")
                            .width(Length::Fixed(180.0))
                            .view()
                    } else if state.pk_is_already_selected(user.public_key()) {
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .text("Select")
                            .width(Length::Fixed(180.0))
                            .view()
                    } else {
                        Button::new()
                            .style(ButtonStyle::Bordered)
                            .text("Select")
                            .width(Length::Fixed(180.0))
                            .on_press(
                                PolicyBuilderMessage::EditSigner(
                                    index,
                                    Box::new(user.clone()),
                                    Box::new(descriptor),
                                )
                                .into(),
                            )
                            .view()
                    })
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill);
                content = content.push(row).push(rule::horizontal());
            }
        }
    }

    // Shared Signers

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
                .push(Space::with_width(Length::Fixed(180.0)))
                .spacing(10)
                .align_items(Alignment::Center)
                .width(Length::Fill),
        )
        .push(rule::horizontal_bold());

    for GetSharedSigner {
        shared_signer_id,
        owner,
        shared_signer,
    } in state.signers.contacts.iter()
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
                .push(Text::new(owner.name()).width(Length::Fill).view())
                .push(if state.is_already_selected(&descriptor) {
                    Button::new()
                        .text("Selected")
                        .width(Length::Fixed(180.0))
                        .view()
                } else if state.pk_is_already_selected(owner.public_key()) {
                    Button::new()
                        .style(ButtonStyle::Bordered)
                        .text("Select")
                        .width(Length::Fixed(180.0))
                        .view()
                } else {
                    Button::new()
                        .style(ButtonStyle::Bordered)
                        .text("Select")
                        .width(Length::Fixed(180.0))
                        .on_press(
                            PolicyBuilderMessage::EditSigner(
                                index,
                                Box::new(owner.clone()),
                                Box::new(descriptor),
                            )
                            .into(),
                        )
                        .view()
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
