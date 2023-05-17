// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fs::File;
use std::io::Write;

use coinstr_sdk::core::proposal::CompletedProposal;
use coinstr_sdk::nostr::EventId;
use coinstr_sdk::util;
use iced::widget::{Column, Row, Space};
use iced::{Command, Element, Length};
use rfd::FileDialog;

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, Text};
use crate::constants::APP_NAME;
use crate::theme::color::{GREEN, GREY, RED};
use crate::theme::icon::{PATCH_CHECK, SAVE, TRASH};

#[derive(Debug, Clone, Default)]
pub enum ProofStatus {
    #[default]
    Unknown,
    Valid(u64),
    Invalid,
}

#[derive(Debug, Clone)]
pub enum CompletedProposalMessage {
    Delete,
    VerifyProof,
    UpdateProofStatus(ProofStatus),
    ExportProof,
    Exported,
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct CompletedProposalState {
    loaded: bool,
    loading: bool,
    completed_proposal_id: EventId,
    completed_proposal: CompletedProposal,
    policy_id: EventId,
    proof_status: ProofStatus,
    error: Option<String>,
}

impl CompletedProposalState {
    pub fn new(
        completed_proposal_id: EventId,
        completed_proposal: CompletedProposal,
        policy_id: EventId,
    ) -> Self {
        Self {
            loaded: false,
            loading: false,
            completed_proposal_id,
            completed_proposal,
            policy_id,
            proof_status: ProofStatus::default(),
            error: None,
        }
    }
}

impl State for CompletedProposalState {
    fn title(&self) -> String {
        format!(
            "{APP_NAME} - Finalized proposal #{}",
            util::cut_event_id(self.completed_proposal_id)
        )
    }

    fn load(&mut self, _ctx: &Context) -> Command<Message> {
        self.loaded = true;
        Command::none()
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::CompletedProposal(msg) = message {
            match msg {
                CompletedProposalMessage::ErrorChanged(error) => {
                    self.loading = false;
                    self.error = error;
                }
                CompletedProposalMessage::VerifyProof => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let completed_proposal = self.completed_proposal.clone();
                    return Command::perform(
                        async move { client.verify_proof(completed_proposal).await },
                        |res| match res {
                            Ok(spendable) => CompletedProposalMessage::UpdateProofStatus(
                                ProofStatus::Valid(spendable),
                            )
                            .into(),
                            Err(_) => {
                                CompletedProposalMessage::UpdateProofStatus(ProofStatus::Invalid)
                                    .into()
                            }
                        },
                    );
                }
                CompletedProposalMessage::UpdateProofStatus(status) => {
                    self.loading = false;
                    self.proof_status = status;
                }
                CompletedProposalMessage::ExportProof => {
                    let path = FileDialog::new()
                        .set_title("Export Proof of Reserve")
                        .set_file_name(&format!(
                            "proof-{}.json",
                            util::cut_event_id(self.completed_proposal_id)
                        ))
                        .save_file();

                    if let Some(path) = path {
                        match self.completed_proposal.export_proof() {
                            Some(proof) => {
                                self.loading = true;
                                return Command::perform(
                                    async move {
                                        let mut file = File::create(path)?;
                                        file.write_all(proof.as_bytes())
                                    },
                                    |res| match res {
                                        Ok(_) => CompletedProposalMessage::Exported.into(),
                                        Err(e) => CompletedProposalMessage::ErrorChanged(Some(
                                            e.to_string(),
                                        ))
                                        .into(),
                                    },
                                );
                            }
                            None => self.error = Some("Not a proof of reserve".to_string()),
                        }
                    }
                }
                CompletedProposalMessage::Exported => {
                    self.loading = false;
                }
                CompletedProposalMessage::Delete => {
                    self.loading = true;
                    let client = ctx.client.clone();
                    let completed_proposal_id = self.completed_proposal_id;
                    return Command::perform(
                        async move {
                            client
                                .delete_completed_proposal_by_id(completed_proposal_id, None)
                                .await
                        },
                        |res| match res {
                            Ok(_) => Message::View(Stage::History),
                            Err(e) => {
                                CompletedProposalMessage::ErrorChanged(Some(e.to_string())).into()
                            }
                        },
                    );
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new()
            .spacing(10)
            .padding(20)
            .push(
                Text::new(format!(
                    "Finalized proposal #{}",
                    util::cut_event_id(self.completed_proposal_id)
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

        let mut buttons = Row::new().spacing(10);

        match &self.completed_proposal {
            CompletedProposal::Spending {
                tx, description, ..
            } => {
                let txid = tx.txid();
                content = content
                    .push(Text::new("Type: spending").view())
                    .push(
                        Text::new(format!("Txid: {txid}"))
                            .on_press(Message::View(Stage::Transaction(txid)))
                            .view(),
                    )
                    .push(Text::new(format!("Description: {description}")).view());
            }
            CompletedProposal::ProofOfReserve { message, .. } => {
                let mut status = Row::new().push(Text::new("Status: ").view());

                match self.proof_status {
                    ProofStatus::Unknown => {
                        status = status.push(Text::new("unknown").color(GREY).view())
                    }
                    ProofStatus::Valid(spendable) => {
                        status = status.push(
                            Text::new(format!(
                                "valid - spendable {} sat",
                                util::format::number(spendable)
                            ))
                            .color(GREEN)
                            .view(),
                        )
                    }
                    ProofStatus::Invalid => {
                        status = status.push(Text::new("invalid").color(RED).view())
                    }
                };

                content = content
                    .push(Text::new("Type: proof-of-reserve").view())
                    .push(Text::new(format!("Message: {message}")).view())
                    .push(status);

                let mut verify_proof_btn = button::border_with_icon(PATCH_CHECK, "Verify proof");
                let mut export_btn = button::border_with_icon(SAVE, "Export");

                if !self.loading {
                    verify_proof_btn =
                        verify_proof_btn.on_press(CompletedProposalMessage::VerifyProof.into());
                    export_btn = export_btn.on_press(CompletedProposalMessage::ExportProof.into());
                }

                buttons = buttons.push(verify_proof_btn).push(export_btn);
            }
        };

        let mut delete_btn = button::danger_with_icon(TRASH, "Delete");

        if !self.loading {
            delete_btn = delete_btn.on_press(CompletedProposalMessage::Delete.into());
        }

        content = content
            .push(Space::with_height(10.0))
            .push(buttons.push(delete_btn))
            .push(Space::with_height(20.0));

        Dashboard::new().view(ctx, content, false, false)
    }
}

impl From<CompletedProposalState> for Box<dyn State> {
    fn from(s: CompletedProposalState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<CompletedProposalMessage> for Message {
    fn from(msg: CompletedProposalMessage) -> Self {
        Self::CompletedProposal(msg)
    }
}
