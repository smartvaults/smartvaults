// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::cmp::Ordering;

use coinstr_core::bdk::TransactionDetails;
use coinstr_core::util::format;
use iced::widget::Column;
use iced::Length;

use crate::app::Message;
use crate::component::{rule, Text};

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
                .cmp(&a.confirmation_time.as_ref().map(|t| t.height))
        });
        if let Some(take) = self.take {
            Box::new(list.into_iter().take(take))
        } else {
            Box::new(list.into_iter())
        }
    }

    pub fn view(self) -> Column<'static, Message> {
        let mut transactions = Column::new()
            .push(Text::new("Transactions").bigger().view())
            .push(rule::horizontal_bold())
            .width(Length::Fill)
            .spacing(10);

        match &self.list {
            Some(list) => {
                if list.is_empty() {
                    transactions = transactions.push(Text::new("No transactions").view());
                } else {
                    for tx in self.list() {
                        let unconfirmed = match &tx.confirmation_time {
                            Some(block_time) => {
                                format!(" - block {}", format::number(block_time.height.into()))
                            }
                            None => String::from(" - unconfirmed"),
                        };
                        let text = match tx.received.cmp(&tx.sent) {
                            Ordering::Greater => Text::new(format!(
                                "Received {} sats{unconfirmed}",
                                format::number(tx.received - tx.sent)
                            )),
                            Ordering::Less => {
                                let fee = match tx.fee {
                                    Some(fee) => format!(" (fee: {} sats)", format::number(fee)),
                                    None => String::new(),
                                };
                                Text::new(format!(
                                    "Sent {} sats{fee}{unconfirmed}",
                                    format::number(tx.sent - tx.received)
                                ))
                            }
                            Ordering::Equal => Text::new(format!("null{unconfirmed}")),
                        };
                        transactions = transactions.push(text.view()).push(rule::horizontal());
                    }
                }
            }
            None => transactions = transactions.push(Text::new("Unavailable").view()),
        };

        transactions
    }
}
