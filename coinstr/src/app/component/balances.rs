// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bdk::Balance;
use coinstr_core::util::format;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Length};

use crate::app::Message;
use crate::component::Text;

pub struct Balances {
    balance: Option<Balance>,
}

impl Balances {
    pub fn new(balance: Option<Balance>) -> Self {
        Self { balance }
    }

    pub fn view(self) -> Row<'static, Message> {
        match self.balance {
            Some(balance) => {
                let pending_balance =
                    balance.untrusted_pending + balance.trusted_pending + balance.immature;

                Row::new()
                    .push(
                        Column::new()
                            .push(Text::new(format::number(balance.confirmed)).size(45).view())
                            .push(if pending_balance > 0 {
                                Text::new(format::number(pending_balance)).size(25).view()
                            } else {
                                Text::new("").size(25).view()
                            })
                            .align_items(Alignment::End),
                    )
                    .push(
                        Column::new()
                            .push(Space::with_height(Length::Fixed(8.0)))
                            .push(Text::new("sats").size(35).view())
                            .push(Space::with_height(Length::Fixed(29.0)))
                            .align_items(Alignment::End),
                    )
                    .spacing(10)
                    .width(Length::Fill)
            }
            None => Row::new()
                .push(Text::new("Unavailabe").view())
                .spacing(10)
                .width(Length::Fill),
        }
    }
}
