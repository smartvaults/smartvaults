// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::db::model::GetTransaction;
use coinstr_sdk::nostr::{EventId, Timestamp};
use coinstr_sdk::util::{self, format};
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Length};

use crate::app::{Context, Message, Stage};
use crate::component::{rule, Button, ButtonStyle, Icon, Text};
use crate::theme::color::{GREEN, RED, YELLOW};
use crate::theme::icon::{BROWSER, CHECK, CLIPBOARD, FULLSCREEN, HOURGLASS};

pub struct TransactionsList {
    list: Vec<GetTransaction>,
    take: Option<usize>,
    policy_id: Option<EventId>,
    hide_policy_id: bool,
}

impl TransactionsList {
    pub fn new(list: Vec<GetTransaction>) -> Self {
        Self {
            list,
            take: None,
            policy_id: None,
            hide_policy_id: false,
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

    pub fn hide_policy_id(self) -> Self {
        Self {
            hide_policy_id: true,
            ..self
        }
    }

    fn list(self) -> Box<dyn Iterator<Item = GetTransaction>> {
        if let Some(take) = self.take {
            Box::new(self.list.into_iter().take(take))
        } else {
            Box::new(self.list.into_iter())
        }
    }

    pub fn view(self, ctx: &Context) -> Column<'static, Message> {
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
                    .push(if self.hide_policy_id {
                        Text::new("").view()
                    } else {
                        Text::new("Policy ID")
                            .bold()
                            .bigger()
                            .width(Length::Fixed(115.0))
                            .view()
                    })
                    .push(
                        Text::new("Date/Time")
                            .bold()
                            .bigger()
                            .width(Length::Fixed(225.0))
                            .view(),
                    )
                    .push(
                        Text::new("Description")
                            .bold()
                            .bigger()
                            .width(Length::FillPortion(2))
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
                    .push(Space::with_width(40.0))
                    .spacing(10)
                    .align_items(Alignment::Center)
                    .width(Length::Fill),
            )
            .push(rule::horizontal_bold())
            .width(Length::Fill)
            .spacing(10);

        if self.list.is_empty() {
            transactions = transactions.push(Text::new("No transactions").extra_light().view());
        } else {
            let list_len = self.list.len();
            let take = self.take;
            let policy_id = self.policy_id;
            let hide_policy_id = self.hide_policy_id;

            for GetTransaction {
                policy_id,
                tx,
                label,
            } in self.list()
            {
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
                    .push(if hide_policy_id {
                        Text::new("").view()
                    } else {
                        Text::new(util::cut_event_id(policy_id))
                            .width(Length::Fixed(115.0))
                            .on_press(Message::View(Stage::Policy(policy_id)))
                            .view()
                    })
                    .push(
                        Text::new(if ctx.hide_balances {
                            String::from("*****")
                        } else {
                            tx.confirmation_time
                                .map(|b| Timestamp::from(b.timestamp).to_human_datetime())
                                .unwrap_or_else(|| String::from("Pending"))
                        })
                        .width(Length::Fixed(225.0))
                        .view(),
                    )
                    .push(
                        Text::new(label.unwrap_or_default())
                            .width(Length::FillPortion(2))
                            .view(),
                    )
                    .push(
                        Text::new(format!(
                            "{} sat",
                            if ctx.hide_balances {
                                String::from("*****")
                            } else {
                                format!(
                                    "{}{}",
                                    if positive { "+" } else { "-" },
                                    format::number(total)
                                )
                            }
                        ))
                        .color(if positive { GREEN } else { RED })
                        .width(Length::Fill)
                        .view(),
                    )
                    .push(
                        Button::new()
                            .icon(CLIPBOARD)
                            .style(ButtonStyle::Bordered)
                            .on_press(Message::Clipboard(tx.txid.to_string()))
                            .width(Length::Fixed(40.0))
                            .view(),
                    )
                    .push({
                        let mut btn = Button::new()
                            .icon(BROWSER)
                            .style(ButtonStyle::Bordered)
                            .width(Length::Fixed(40.0));

                        if let Ok(url) = ctx.client.config().block_explorer() {
                            btn = btn
                                .on_press(Message::OpenInBrowser(format!("{url}/tx/{}", tx.txid)));
                        }

                        btn.view()
                    })
                    .push(
                        Button::new()
                            .icon(FULLSCREEN)
                            .on_press(Message::View(Stage::Transaction {
                                policy_id,
                                txid: tx.txid,
                            }))
                            .width(Length::Fixed(40.0))
                            .view(),
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
        };

        transactions
    }
}
