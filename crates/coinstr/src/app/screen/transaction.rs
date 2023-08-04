// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bitcoin::{Address, Txid};
use coinstr_sdk::db::model::GetTransaction;
use coinstr_sdk::nostr::{EventId, Timestamp};
use coinstr_sdk::util::{self, format};
use iced::widget::{Column, Row, Space};
use iced::{Command, Element, Length};

use crate::app::component::Dashboard;
use crate::app::{Context, Message, Stage, State};
use crate::component::{rule, Text};
use crate::theme::color::{GREEN, RED};

#[derive(Debug, Clone)]
pub enum TransactionMessage {
    LoadTx(GetTransaction),
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
            async move { client.get_tx(policy_id, txid).ok() },
            |res| match res {
                Some(tx) => TransactionMessage::LoadTx(tx).into(),
                None => Message::View(Stage::Transactions(None)),
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
                    self.tx = Some(tx);
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

            let (inputs, outputs) = if let Some(transaction) = &tx.transaction {
                let mut inputs = Column::new()
                    .push(
                        Text::new(format!("{} inputs", transaction.input.len()))
                            .bold()
                            .size(20)
                            .view(),
                    )
                    .push(rule::horizontal_bold());

                for txin in transaction.input.iter() {
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
                        Text::new(format!("{} outputs", transaction.output.len()))
                            .bold()
                            .size(20)
                            .view(),
                    )
                    .push(rule::horizontal_bold());

                for txout in transaction.output.iter() {
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
                                .push(
                                    Text::new(format!("{} sat", format::number(txout.value)))
                                        .extra_light()
                                        .view(),
                                )
                                .spacing(5),
                        )
                        .push(rule::horizontal());
                }

                (inputs, outputs)
            } else {
                (
                    Column::new().push(Text::new("Inputs unavailable").bold().size(20).view()),
                    Column::new().push(Text::new("Outputs unavailable").bold().size(20).view()),
                )
            };

            let txid: String = self.txid.to_string();
            let title = format!("Txid {}..{}", &txid[..6], &txid[txid.len() - 6..]);

            let (confirmed_at_block, confirmed_at_time, confirmations) =
                match tx.confirmation_time.as_ref() {
                    Some(block_time) => {
                        let confirmations: u32 =
                            ctx.client.block_height().saturating_sub(block_time.height) + 1;
                        (
                            format::number(block_time.height as u64),
                            Timestamp::from(block_time.timestamp).to_human_datetime(),
                            format::number(confirmations as u64),
                        )
                    }
                    None => (
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
                                .push(Text::new("Block").bigger().extra_light().view())
                                .push(Text::new(confirmed_at_block).bigger().view())
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Confirmations").bigger().extra_light().view())
                                .push(Text::new(confirmations).bigger().view())
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Lock time").bigger().extra_light().view())
                                .push(
                                    Text::new(
                                        tx.transaction
                                            .as_ref()
                                            .map(|t| format::number(t.lock_time.to_u32() as u64))
                                            .unwrap_or_else(|| "00000000".to_string()),
                                    )
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
                                .push(Text::new("Incoming").bigger().extra_light().view())
                                .push(
                                    Text::new(format!("{} sat", format::number(tx.received)))
                                        .bigger()
                                        .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Outcoming").bigger().extra_light().view())
                                .push(
                                    Text::new(format!("{} sat", format::number(tx.sent)))
                                        .bigger()
                                        .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Net").bigger().extra_light().view())
                                .push(
                                    Text::new(format!(
                                        "{}{} sat",
                                        if positive { "+" } else { "-" },
                                        format::number(total)
                                    ))
                                    .color(if positive { GREEN } else { RED })
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
                                .push(Text::new("Fee").bigger().extra_light().view())
                                .push(
                                    Text::new(match tx.fee {
                                        Some(fee) => format!("{} sat", format::number(fee)),
                                        None => String::from("-"),
                                    })
                                    .color(RED)
                                    .bigger()
                                    .view(),
                                )
                                .spacing(10)
                                .width(Length::Fill),
                        )
                        .push(
                            Column::new()
                                .push(Text::new("Date/Time").bigger().extra_light().view())
                                .push(Text::new(confirmed_at_time).bigger().view())
                                .spacing(10)
                                .width(Length::FillPortion(2)),
                        ),
                )
                .push(
                    Row::new().push(
                        Column::new()
                            .push(Text::new("Description").bigger().extra_light().view())
                            .push(
                                Text::new(label.as_ref().map(|s| s.as_str()).unwrap_or_default())
                                    .bigger()
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
