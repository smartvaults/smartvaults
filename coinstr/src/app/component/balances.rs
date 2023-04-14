// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bdk::Balance;
use coinstr_core::util::format;
use iced::widget::{Column, Row, Space};
use iced::{Alignment, Length};

use crate::app::Message;
use crate::component::Text;

pub struct Balances {
    balance: Balance,
}

impl Balances {
    pub fn new(balance: Balance) -> Self {
        Self { balance }
    }

    pub fn view(self) -> Row<'static, Message> {
        let pending_balance =
            self.balance.untrusted_pending + self.balance.trusted_pending + self.balance.immature;

        Row::new()
            .push(
                Column::new()
                    .push(
                        Text::new(format::number(self.balance.confirmed))
                            .size(45)
                            .view(),
                    )
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
}
