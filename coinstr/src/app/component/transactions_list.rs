// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bdk::TransactionDetails;
use coinstr_core::util::format;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Length};

use crate::app::Message;
use crate::component::{button, rule, Icon, Text};
use crate::theme::color::{GREEN, YELLOW};
use crate::theme::icon::{CHECK, FULLSCREEN, HOURGLASS};

pub struct TransactionsList {
    list: Option<Vec<TransactionDetails>>,
    take: Option<usize>,
}

impl TransactionsList {
    pub fn new(list: Option<Vec<TransactionDetails>>) -> Self {
        Self { list, take: None }
    }

    pub fn take(self, num: usize) -> Self {
        Self {
            take: Some(num),
            ..self
        }
    }

    fn list(self) -> Box<dyn Iterator<Item = TransactionDetails>> {
        let mut list = self.list.unwrap_or_default();
        list.sort_by(|a, b| {
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
                        Text::new("Timestamp")
                            .bold()
                            .bigger()
                            .width(Length::Fixed(125.0))
                            .view(),
                    )
                    .push(
                        Text::new("Incoming")
                            .bold()
                            .bigger()
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(
                        Text::new("Outcoming")
                            .bold()
                            .bigger()
                            .width(Length::Fill)
                            .view(),
                    )
                    .push(
                        Text::new("Total")
                            .bold()
                            .bigger()
                            .width(Length::Fill)
                            .view(),
                    )
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
                    transactions = transactions.push(Text::new("No transactions").view());
                } else {
                    for tx in self.list() {
                        let status = if tx.confirmation_time.is_some() {
                            Icon::new(CHECK).color(GREEN)
                        } else {
                            Icon::new(HOURGLASS).color(YELLOW)
                        };

                        let (total, positive): (u64, bool) = {
                            let received: i64 = tx.received as i64;
                            let sent: i64 = tx.sent as i64;
                            let tot = received.saturating_sub(sent);
                            let positive = tot >= 0;
                            (tot as u64, positive)
                        };

                        let row = Row::new()
                            .push(status.width(Length::Fixed(70.0)).view())
                            .push(
                                Text::new(
                                    tx.confirmation_time
                                        .map(|b| b.timestamp.to_string())
                                        .unwrap_or_default(),
                                )
                                .width(Length::Fixed(125.0))
                                .view(),
                            )
                            .push(
                                Text::new(format!("{} sats", format::number(tx.received)))
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new(format!("{} sats", format::number(tx.sent)))
                                    .width(Length::Fill)
                                    .view(),
                            )
                            .push(
                                Text::new(format!(
                                    "{}{} sats",
                                    if positive { "+" } else { "-" },
                                    format::number(total)
                                ))
                                .width(Length::Fill)
                                .view(),
                            )
                            .push(button::primary_only_icon(FULLSCREEN).width(Length::Fixed(40.0)))
                            .spacing(10)
                            .align_items(Alignment::Center)
                            .width(Length::Fill);
                        transactions = transactions.push(row).push(rule::horizontal());
                    }
                }
            }
            None => transactions = transactions.push(Text::new("Unavailable").view()),
        };

        transactions
    }
}
