// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::time::Duration;

use coinstr_core::bdk::blockchain::ElectrumBlockchain;
use coinstr_core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_core::bitcoin::{Network, Txid, XOnlyPublicKey};
use coinstr_core::nostr_sdk::{EventId, Timestamp};
use coinstr_core::proposal::SpendingProposal;
use coinstr_core::util;
use iced::widget::{Column, Row, Space};
use iced::{time, Alignment, Command, Element, Length, Subscription};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{button, rule, Text};
use crate::constants::APP_NAME;
use crate::theme::color::{GREEN, RED, YELLOW};

#[derive(Debug, Clone)]
pub enum ProposalMessage {
    Approve,
    Broadcast,
    Signed(bool),
    Reload,
    CheckPsbts,
    LoadApprovedProposals(
        BTreeMap<XOnlyPublicKey, (EventId, PartiallySignedTransaction, Timestamp)>,
    ),
    ErrorChanged(Option<String>),
}

#[derive(Debug)]
pub struct ProposalState {
    loading: bool,
    loaded: bool,
    signed: bool,
    proposal_id: EventId,
    proposal: SpendingProposal,
    approved_proposals: BTreeMap<XOnlyPublicKey, (EventId, PartiallySignedTransaction, Timestamp)>,
    error: Option<String>,
}

impl ProposalState {
    pub fn new(proposal_id: EventId, proposal: SpendingProposal) -> Self {
        Self {
            loading: false,
            loaded: false,
            signed: false,
            proposal_id,
            proposal,
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

    fn subscription(&self) -> Subscription<Message> {
        Subscription::batch(vec![
            time::every(Duration::from_secs(10)).map(|_| ProposalMessage::Reload.into()),
            time::every(Duration::from_secs(30)).map(|_| ProposalMessage::CheckPsbts.into()),
        ])
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let cache = ctx.cache.clone();
        let proposal_id = self.proposal_id;
        self.loading = true;
        Command::perform(
            async move {
                if cache.proposal_exists(proposal_id).await {
                    Some(
                        cache
                            .signed_psbts_by_proposal_id(proposal_id)
                            .await
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
                    let cache = ctx.cache.clone();
                    let proposal_id = self.proposal_id;
                    return Command::perform(
                        async move {
                            let (event, approved_proposal) =
                                client.approve(proposal_id, None).await?;
                            cache
                                .cache_approved_proposal(
                                    proposal_id,
                                    client.keys().public_key(),
                                    event.id,
                                    approved_proposal.psbt(),
                                    event.created_at,
                                )
                                .await;
                            Ok::<(), Box<dyn std::error::Error>>(())
                        },
                        |res| match res {
                            Ok(_) => ProposalMessage::Reload.into(),
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ProposalMessage::Broadcast => {
                    self.loading = true;

                    let client = ctx.client.clone();
                    let cache = ctx.cache.clone();
                    let proposal_id = self.proposal_id;

                    // TODO: get electrum endpoint from config file
                    let bitcoin_endpoint: &str = match ctx.coinstr.network() {
                        Network::Bitcoin => "ssl://blockstream.info:700",
                        Network::Testnet => "ssl://blockstream.info:993",
                        _ => panic!("Endpoints not availabe for this network"),
                    };

                    return Command::perform(
                        async move {
                            let blockchain =
                                ElectrumBlockchain::from(ElectrumClient::new(bitcoin_endpoint)?);
                            let txid = client.broadcast(proposal_id, &blockchain, None).await?;
                            cache.uncache_proposal(proposal_id).await;
                            cache.sync_wallets(&blockchain, None, true).await?;
                            Ok::<Txid, Box<dyn std::error::Error>>(txid)
                        },
                        |res| match res {
                            Ok(txid) => Message::View(Stage::Transaction(txid)),
                            Err(e) => ProposalMessage::ErrorChanged(Some(e.to_string())).into(),
                        },
                    );
                }
                ProposalMessage::Signed(value) => self.signed = value,
                ProposalMessage::Reload => return self.load(ctx),
                ProposalMessage::CheckPsbts => {
                    if !self.signed {
                        let client = ctx.client.clone();
                        let base_psbt = self.proposal.psbt.clone();
                        let signed_psbts = self
                            .approved_proposals
                            .iter()
                            .map(|(_, (_, psbt, ..))| psbt.clone())
                            .collect();
                        return Command::perform(
                            async move {
                                client.combine_psbts(base_psbt, signed_psbts)?;
                                Ok::<(), Box<dyn std::error::Error>>(())
                            },
                            |res| match res {
                                Ok(_) => ProposalMessage::Signed(true).into(),
                                Err(_) => ProposalMessage::Signed(false).into(),
                            },
                        );
                    }
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(10).padding(20);
        let mut center_y = true;
        let mut center_x = true;

        if self.loaded {
            center_y = false;
            center_x = false;
            let title = format!("Proposal #{}", util::cut_event_id(self.proposal_id));
            content = content
                .push(Text::new(title).size(40).bold().view())
                .push(Space::with_height(Length::Fixed(40.0)))
                .push(Text::new(format!("Address: {}", self.proposal.to_address)).view())
                .push(
                    Text::new(format!(
                        "Amount: {} sat",
                        util::format::number(self.proposal.amount)
                    ))
                    .view(),
                )
                .push(Text::new(format!("Memo: {}", &self.proposal.memo)).view());

            let mut status = Row::new().push(Text::new("Status: ").view());

            if self.signed {
                status = status.push(Text::new("signed").color(GREEN).view());
            } else {
                status = status.push(Text::new("unsigned").color(YELLOW).view());
            }

            content = content.push(status);

            let (approve_btn, mut broadcast_btn) =
                match self.approved_proposals.get(&ctx.client.keys().public_key()) {
                    Some(_) => {
                        let approve_btn = button::border("Approve");
                        let broadcast_btn = button::primary("Broadcast");
                        (approve_btn, broadcast_btn)
                    }
                    None => {
                        let mut approve_btn = button::primary("Approve");
                        let broadcast_btn = button::border("Broadcast");

                        if !self.loading {
                            approve_btn = approve_btn.on_press(ProposalMessage::Approve.into());
                        }

                        (approve_btn, broadcast_btn)
                    }
                };

            if self.signed && !self.loading {
                broadcast_btn = broadcast_btn.on_press(ProposalMessage::Broadcast.into());
            }

            content = content
                .push(Space::with_height(10.0))
                .push(Row::new().push(approve_btn).push(broadcast_btn).spacing(10))
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
        } else {
            content = content.push(Text::new("Loading...").view());
        }

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
