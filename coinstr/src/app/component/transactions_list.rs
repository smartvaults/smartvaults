// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bdk::TransactionDetails;
use coinstr_core::nostr_sdk::{EventId, Timestamp};
use coinstr_core::util::format;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Length};

use crate::app::cache::Transactions;
use crate::app::{Message, Stage};
use crate::component::{button, rule, Icon, Text};
use crate::theme::color::{GREEN, RED, YELLOW};
use crate::theme::icon::{CHECK, CLIPBOARD, FULLSCREEN, HOURGLASS};

pub struct TransactionsList {
    list: Option<Transactions>,
    take: Option<usize>,
    policy_id: Option<EventId>,
}

impl TransactionsList {
    pub fn new(list: Option<Transactions>) -> Self {
        Self {
            list,
            take: None,
            policy_id: None,
        }
    }

    pub fn take(self, num: usize) -> Self {
        Self {
            take: Some(num),
            ..self
        }
    }

    pub fn policy_id(self, policy_id: EventId) -> Self {
        Self {
            policy_id: Some(policy_id),
            ..self
        }
    }

    fn list(self) -> Box<dyn Iterator<Item = (TransactionDetails, Option<String>)>> {
        let mut list = self.list.unwrap_or_default();
        list.sort_by(|(a, _), (b, _)| {
            b.confirmation_time
                .as_ref()
                .map(|t| t.height)
                .unwrap_or(u32::MAX)
                .cmp(
                    &a.confirmation_time
                        .as_ref()
                        .map(|t| t.height)
                        .unwrap_or(u32::MAX),
                )
        });
        if let Some(take) = self.take {
            Box::new(list.into_iter().take(take))
        } else {
            Box::new(list.into_iter())
        }
    }

    pub fn view(self) -> Column<'static, Message> {
        let mut transactions = Column::new()
            .push(
                Row::new()
                    .push(
                        Text::new("Status")
                            .bold()
                            .bigger()
                            .width(Length::Fixed(70.0))
                            .view(),
                    )
                    .push(
                        Text::new("Date/Time")
                            .bold()
                            .bigger()
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(
                        Text::new("Description")
                            .bold()
                            .bigger()
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(
                        Text::new("Amount")
                            .bold()
                            .bigger()
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(Space::with_width(40.0))
                    .push(Space::with_width(40.0))
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(rule::horizontal_bold())
            .width(Length::Fill)
            .spacing(10);

        match &self.list {
            Some(list) => {
                if list.is_empty() {
                    transactions =
                        transactions.push(Text::new("No transactions").extra_light().view());
                } else {
                    let list_len = list.len();
                    let take = self.take;
                    let policy_id = self.policy_id;

                    for (tx, description) in self.list() {
                        let status = if tx.confirmation_time.is_some() {
                            Icon::new(CHECK).color(GREEN)
                        } else {
                            Icon::new(HOURGLASS).color(YELLOW)
                        };

                        let (total, positive): (u64, bool) = {
                            let received: i64 = tx.received as i64;
                            let sent: i64 = tx.sent as i64;
                            let tot = received - sent;
                            let positive = tot >= 0;
                            (tot.unsigned_abs(), positive)
                        };

                        let row = Row::new()
                            .push(status.width(Length::Fixed(70.0)).view())
                            .push(
                                Text::new(
                                    tx.confirmation_time
                                        .map(|b| Timestamp::from(b.timestamp).to_human_datetime())
                                        .unwrap_or_default(),
                                )
                                .width(Length::Fill)
                                .view(),
                            )
                            .push(
                                Text::new(description.unwrap_or_default())
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new(format!(
                                    "{}{} sat",
                                    if positive { "+" } else { "-" },
                                    format::number(total)
                                ))
                                .color(if positive { GREEN } else { RED })
                                .width(Length::Fill)
                                .view(),
                            )
                            .push(
                                button::border_only_icon(CLIPBOARD)
                                    .on_press(Message::Clipboard(tx.txid.to_string()))
                                    .width(Length::Fixed(40.0)),
                            )
                            .push(
                                button::primary_only_icon(FULLSCREEN)
                                    .on_press(Message::View(Stage::Transaction(tx.txid)))
                                    .width(Length::Fixed(40.0)),
                            )
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill);
                        transactions = transactions.push(row).push(rule::horizontal());
                    }

                    if let Some(take) = take {
                        if list_len > take {
                            transactions = transactions.push(
                                Text::new("Show all")
                                    .on_press(Message::View(Stage::Transactions(policy_id)))
                                    .view(),
                            );
                        }
                    }
                }
            }
            None => transactions = transactions.push(Text::new("Unavailable").view()),
        };

        transactions
    }
}
