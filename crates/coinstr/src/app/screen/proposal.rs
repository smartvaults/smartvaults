// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;

use coinstr_sdk::core::bitcoin::XOnlyPublicKey;
use coinstr_sdk::core::proposal::Proposal;
use coinstr_sdk::core::types::Psbt;
use coinstr_sdk::core::{ApprovedProposal, CompletedProposal};
use coinstr_sdk::nostr::{EventId, Timestamp};
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::constants::APP_NAME;
use crate::theme::color::{GREEN, RED, YELLOW};
use crate::theme::icon::{SAVE, TRASH};

#[derive(Debug, Clone)]
pub enum ProposalMessage {
    Approve,
    Finalize,
    Signed(bool),
    Reload,
    CheckPsbts,
    LoadApprovedProposals(BTreeMap<XOnlyPublicKey, (EventId, ApprovedProposal, Timestamp)>),
    ExportPsbt,
    Delete,
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct ProposalState {
    loading: bool,
    loaded: bool,
    signed: bool,
    proposal_id: EventId,
    proposal: Proposal,
    policy_id: EventId,
    approved_proposals: BTreeMap<XOnlyPublicKey, (EventId, ApprovedProposal, Timestamp)>,
    error: Option<String>,
}

impl ProposalState {
    pub fn new(proposal_id: EventId, proposal: Proposal, policy_id: EventId) -> Self {
        Self {
            loading: false,
            loaded: false,
            signed: false,
            proposal_id,
            proposal,
            policy_id,
            approved_proposals: BTreeMap::new(),
            error: None,
        }
    }
}

impl State for ProposalState {
    fn title(&self) -> String {
        format!(
            "{APP_NAME} - Proposal #{}",
            util::cut_event_id(self.proposal_id)
        )
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        let proposal_id = self.proposal_id;
        self.loading = true;
        Command::perform(
            async move {
                if client.db.proposal_exists(proposal_id).ok()? {
                    Some(
                        client
                            .db
                            .signed_psbts_by_proposal_id(proposal_id)
                            .unwrap_or_default(),
                    )
                } else {
                    None
                }
            },
            |res| match res {
                Some(data) => ProposalMessage::LoadApprovedProposals(data).into(),
                None => Message::View(Stage::Dashboard),
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Proposal(msg) = message {
            match msg {
                ProposalMessage::LoadApprovedProposals(value) => {
                    self.approved_proposals = value;
                    self.loading = false;
                    self.loaded = true;
                    return Command::perform(async {}, |_| ProposalMessage::CheckPsbts.into());
                }
                ProposalMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                ProposalMessage::Approve => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let proposal_id = self.proposal_id;
                    return Command::perform(
                        async move { client.approve(proposal_id, None).await },
                        |res| match res {
                            Ok(_) => ProposalMessage::Reload.into(),
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ProposalMessage::Finalize => {
                    self.loading = true;

                    let client = ctx.client.clone();
                    let proposal_id = self.proposal_id;

                    return Command::perform(
                        async move { client.finalize(proposal_id, None).await },
                        |res| match res {
                            Ok(proposal) => match proposal {
                                CompletedProposal::Spending { tx, .. } => {
                                    Message::View(Stage::Transaction(tx.txid()))
                                }
                                CompletedProposal::ProofOfReserve { .. } => {
                                    Message::View(Stage::History)
                                }
                            },
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ProposalMessage::Signed(value) => self.signed = value,
                ProposalMessage::Reload => return self.load(ctx),
                ProposalMessage::CheckPsbts => {
                    if !self.signed {
                        let client = ctx.client.clone();
                        let proposal = self.proposal.clone();
                        let approved_proposals = self
                            .approved_proposals
                            .iter()
                            .map(|(_, (_, p, ..))| p.clone())
                            .collect();
                        return Command::perform(
                            async move { proposal.finalize(approved_proposals, client.network()) },
                            |res| match res {
                                Ok(_) => ProposalMessage::Signed(true).into(),
                                Err(_) => ProposalMessage::Signed(false).into(),
                            },
                        );
                    }
                }
                ProposalMessage::ExportPsbt => {
                    let path = FileDialog::new()
                        .set_title("Export PSBT")
                        .set_file_name(&format!(
                            "proposal-{}.psbt",
                            util::cut_event_id(self.proposal_id)
                        ))
                        .save_file();

                    if let Some(path) = path {
                        let psbt = self.proposal.psbt();
                        match psbt.save_to_file(&path) {
                            Ok(_) => {
                                log::info!("PSBT exported to {}", path.display())
                            }
                            Err(e) => log::error!("Impossible to create file: {e}"),
                        }
                    }
                }
                ProposalMessage::Delete => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let proposal_id = self.proposal_id;
                    return Command::perform(
                        async move { client.delete_proposal_by_id(proposal_id, None).await },
                        |res| match res {
                            Ok(_) => Message::View(Stage::Proposals),
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut center_y = true;
        let mut center_x = true;

        let content = if self.loaded {
            center_y = false;
            center_x = false;

            let mut content = Column::new()
                .spacing(10)
                .padding(20)
                .push(
                    Text::new(format!(
                        "Proposal #{}",
                        util::cut_event_id(self.proposal_id)
                    ))
                    .size(40)
                    .bold()
                    .view(),
                )
                .push(Space::with_height(Length::Fixed(40.0)))
                .push(
                    Text::new(format!("Policy ID: {}", util::cut_event_id(self.policy_id)))
                        .on_press(Message::View(Stage::Policy(self.policy_id)))
                        .view(),
                );

            let finalize_btn_text: &str = match &self.proposal {
                Proposal::Spending {
                    to_address,
                    amount,
                    description,
                    ..
                } => {
                    content = content
                        .push(Text::new("Type: spending").view())
                        .push(Text::new(format!("Address: {to_address}")).view())
                        .push(
                            Text::new(format!("Amount: {} sat", util::format::number(*amount)))
                                .view(),
                        )
                        .push(Text::new(format!("Description: {description}")).view());

                    "Broadcast"
                }
                Proposal::ProofOfReserve { message, .. } => {
                    content = content
                        .push(Text::new("Type: proof-of-reserve").view())
                        .push(Text::new(format!("Message: {message}")).view());

                    "Finalize"
                }
            };

            let mut status = Row::new().push(Text::new("Status: ").view());

            if self.signed {
                status = status.push(Text::new("signed").color(GREEN).view());
            } else {
                status = status.push(Text::new("unsigned").color(YELLOW).view());
            }

            content = content.push(status);

            let (approve_btn, mut finalize_btn) =
                match self.approved_proposals.get(&ctx.client.keys().public_key()) {
                    Some(_) => {
                        let approve_btn = button::border("Approve");
                        let finalize_btn = button::primary(finalize_btn_text);
                        (approve_btn, finalize_btn)
                    }
                    None => {
                        let mut approve_btn = button::primary("Approve");
                        let finalize_btn = button::border(finalize_btn_text);

                        if !self.loading {
                            approve_btn = approve_btn.on_press(ProposalMessage::Approve.into());
                        }

                        (approve_btn, finalize_btn)
                    }
                };

            if self.signed && !self.loading {
                finalize_btn = finalize_btn.on_press(ProposalMessage::Finalize.into());
            }

            let mut export_btn = button::border_with_icon(SAVE, "Export PSBT");
            let mut delete_btn = button::danger_with_icon(TRASH, "Delete");

            if !self.loading {
                export_btn = export_btn.on_press(ProposalMessage::ExportPsbt.into());
                delete_btn = delete_btn.on_press(ProposalMessage::Delete.into());
            }

            content = content
                .push(Space::with_height(10.0))
                .push(
                    Row::new()
                        .push(approve_btn)
                        .push(finalize_btn)
                        .push(export_btn)
                        .push(delete_btn)
                        .spacing(10),
                )
                .push(Space::with_height(20.0));

            if let Some(error) = &self.error {
                content = content.push(Text::new(error).color(RED).view());
            };

            if !self.approved_proposals.is_empty() {
                content = content
                    .push(Text::new("Approvals").bold().bigger().view())
                    .push(Space::with_height(10.0))
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
                                Text::new("Date/Time")
                                    .bold()
                                    .bigger()
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(Text::new("User").bold().bigger().width(Length::Fill).view())
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill),
                    )
                    .push(rule::horizontal_bold());

                for (author, (event_id, _, timestamp)) in self.approved_proposals.iter() {
                    let row = Row::new()
                        .push(
                            Text::new(util::cut_event_id(*event_id))
                                .width(Length::Fixed(115.0))
                                .view(),
                        )
                        .push(
                            Text::new(timestamp.to_human_datetime())
                                .width(Length::Fill)
                                .view(),
                        )
                        .push(
                            Text::new(cut_public_key(*author))
                                .width(Length::Fill)
                                .view(),
                        )
                        .spacing(10)
                        .align_items(Alignment::Center)
                        .width(Length::Fill);
                    content = content.push(row).push(rule::horizontal());
                }
            }

            content
        } else {
            Column::new().push(Text::new("Loading...").view())
        };

        Dashboard::new().view(ctx, content, center_x, center_y)
    }
}

impl From<ProposalState> for Box<dyn State> {
    fn from(s: ProposalState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<ProposalMessage> for Message {
    fn from(msg: ProposalMessage) -> Self {
        Self::Proposal(msg)
    }
}

fn cut_public_key(pk: XOnlyPublicKey) -> String {
    let pk = pk.to_string();
    format!("{}:{}", &pk[0..8], &pk[pk.len() - 8..])
}
