// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row, Space};
use iced::{Command, Element, Length};
use smartvaults_sdk::core::bdk::chain::ConfirmationTime;
use smartvaults_sdk::core::bitcoin::{Address, Txid};
use smartvaults_sdk::nostr::{EventId, Timestamp};
use smartvaults_sdk::types::GetTransaction;
use smartvaults_sdk::util::{self, format};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Amount, AmountSign, Text};

#[derive(Debug, Clone)]
pub enum TransactionMessage {
    LoadTx(Box<GetTransaction>),
    Reload,
}

#[derive(Debug)]
pub struct TransactionState {
    loading: bool,
    loaded: bool,
    policy_id: EventId,
    txid: Txid,
    tx: Option<GetTransaction>,
}

impl TransactionState {
    pub fn new(policy_id: EventId, txid: Txid) -> Self {
        Self {
            loading: false,
            loaded: false,
            policy_id,
            txid,
            tx: None,
        }
    }
}

impl State for TransactionState {
    fn title(&self) -> String {
        format!("Tx #{}", util::cut_txid(self.txid))
    }

    fn load(&mut self, ctx: &Context) -> Command<Message> {
        let client = ctx.client.clone();
        let txid = self.txid;
        let policy_id = self.policy_id;
        self.loading = true;
        Command::perform(
            async move { client.get_tx(policy_id, txid).await.ok() },
            |res| match res {
                Some(tx) => TransactionMessage::LoadTx(Box::new(tx)).into(),
                None => Message::View(Stage::Activity),
            },
        )
    }

    fn update(&mut self, ctx: &mut Context, message: Message) -> Command<Message> {
        if !self.loaded && !self.loading {
            return self.load(ctx);
        }

        if let Message::Transaction(msg) = message {
            match msg {
                TransactionMessage::LoadTx(tx) => {
                    self.tx = Some(*tx);
                    self.loading = false;
                    self.loaded = true;
                }
                TransactionMessage::Reload => {
                    return self.load(ctx);
                }
            }
        }

        Command::none()
    }

    fn view(&self, ctx: &Context) -> Element<Message> {
        let mut content = Column::new().spacing(20).padding(20);

        if let Some(GetTransaction { tx, label, .. }) = &self.tx {
            let (total, positive): (u64, bool) = {
                let received: i64 = tx.received as i64;
                let sent: i64 = tx.sent as i64;
                let tot = received - sent;
                let positive = tot >= 0;
                (tot.unsigned_abs(), positive)
            };

            let (inputs, outputs) = {
                let mut inputs = Column::new()
                    .push(
                        Text::new(format!("{} inputs", tx.input.len()))
                            .bold()
                            .size(20)
                            .view(),
                    )
                    .push(rule::horizontal_bold());

                for txin in tx.input.iter() {
                    let txid: String = txin.previous_output.txid.to_string();
                    inputs = inputs
                        .push(
                            Column::new()
                                .push(
                                    Text::new(format!(
                                        "{}..{}:{}",
                                        &txid[..8],
                                        &txid[txid.len() - 8..],
                                        txin.previous_output.vout
                                    ))
                                    .view(),
                                )
                                .spacing(5),
                        )
                        .push(rule::horizontal());
                }

                let mut outputs = Column::new()
                    .push(
                        Text::new(format!("{} outputs", tx.output.len()))
                            .bold()
                            .size(20)
                            .view(),
                    )
                    .push(rule::horizontal_bold());

                for txout in tx.output.iter() {
                    outputs = outputs
                        .push(
                            Column::new()
                                .push(
                                    Text::new(
                                        Address::from_script(
                                            &txout.script_pubkey,
                                            ctx.client.network(),
                                        )
                                        .map(|a| {
                                            let a = a.to_string();
                                            format!("{}..{}", &a[..8], &a[a.len() - 8..])
                                        })
                                        .unwrap_or_else(|_| "Error".to_string()),
                                    )
                                    .view(),
                                )
                                .push(Amount::new(txout.value).bold().view())
                                .spacing(5),
                        )
                        .push(rule::horizontal());
                }

                (inputs, outputs)
            };

            let txid: String = self.txid.to_string();
            let title = format!("Txid {}..{}", &txid[..6], &txid[txid.len() - 6..]);

            let (confirmed_at_block, confirmed_at_time, confirmations) = match tx.confirmation_time
            {
                ConfirmationTime::Confirmed { height, time } => {
                    let confirmations: u32 = ctx.client.block_height().saturating_sub(height) + 1;
                    (
                        format::number(height as u64),
                        Timestamp::from(time).to_human_datetime(),
                        format::number(confirmations as u64),
                    )
                }
                ConfirmationTime::Unconfirmed { .. } => (
                    "Unconfirmed".to_string(),
                    "Unconfirmed".to_string(),
                    "Unconfirmed".to_string(),
                ),
            };

            content = content
                .push(Text::new(title).size(40).bold().view())
                .push(Space::with_height(Length::Fixed(10.0)))
                .push(
                    Row::new()
                        .push(
                            Column::new()
                                .push(Text::new("Block").big().extra_light().view())
                                .push(Text::new(confirmed_at_block).big().view())
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Confirmations").big().extra_light().view())
                                .push(Text::new(confirmations).big().view())
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Lock time").big().extra_light().view())
                                .push(
                                    Text::new(format::number(
                                        tx.lock_time.to_consensus_u32() as u64
                                    ))
                                    .big()
                                    .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(
                    Row::new()
                        .push(
                            Column::new()
                                .push(Text::new("Incoming").big().extra_light().view())
                                .push(Amount::new(tx.received).bold().bigger().view())
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Outcoming").big().extra_light().view())
                                .push(Amount::new(tx.sent).bold().bigger().view())
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Net").big().extra_light().view())
                                .push(
                                    Amount::new(total)
                                        .sign(if positive {
                                            AmountSign::Positive
                                        } else {
                                            AmountSign::Negative
                                        })
                                        .bold()
                                        .bigger()
                                        .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .spacing(10)
                        .width(Length::Fill),
                )
                .push(
                    Row::new()
                        .push(
                            Column::new()
                                .push(Text::new("Fee (amount)").big().extra_light().view())
                                .push(match tx.fee.amount {
                                    Some(fee) => Amount::new(fee).bold().bigger().view(),
                                    None => Row::new().push(Text::new("Unknown").big().view()),
                                })
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Fee (rate)").big().extra_light().view())
                                .push(match tx.fee.rate {
                                    Some(fee) => {
                                        Text::new(format!("{:.2} sat/vB", fee.as_sat_per_vb()))
                                            .big()
                                            .view()
                                    }
                                    None => Text::new("Unknown").big().view(),
                                })
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Date/Time").big().extra_light().view())
                                .push(Text::new(confirmed_at_time).big().view())
                                .spacing(10)
                                .width(Length::Fill),
                        ),
                )
                .push(
                    Row::new().push(
                        Column::new()
                            .push(Text::new("Description").big().extra_light().view())
                            .push(
                                Text::new(label.as_ref().map(|s| s.as_str()).unwrap_or_default())
                                    .big()
                                    .view(),
                            )
                            .spacing(10)
                            .width(Length::Fill),
                    ),
                )
                .push(Space::with_height(Length::Fixed(10.0)))
                .push(
                    Row::new()
                        .push(inputs.spacing(10).width(Length::Fill))
                        .push(outputs.spacing(10).width(Length::Fill))
                        .spacing(50)
                        .width(Length::Fill),
                )
        }

        Dashboard::new()
            .loaded(self.loaded && self.tx.is_some())
            .view(ctx, content, false, false)
    }
}

impl From<TransactionState> for Box<dyn State> {
    fn from(s: TransactionState) -> Box<dyn State> {
        Box::new(s)
    }
}

impl From<TransactionMessage> for Message {
    fn from(msg: TransactionMessage) -> Self {
        Self::Transaction(msg)
    }
}
