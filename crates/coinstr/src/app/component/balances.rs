// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bdk::Balance;
use coinstr_core::util::format;
use iced::widget::{Column, Row};
use iced::{Alignment, Length};

use crate::app::Message;
use crate::component::{button, Text};
use crate::theme::icon::{ARROW_DOWN, ARROW_UP};

pub struct Balances {
    balance: Option<Balance>,
    size: u16,
    on_send: Option<Message>,
    on_deposit: Option<Message>,
}

impl Balances {
    pub fn new(balance: Option<Balance>) -> Self {
        Self {
            balance,
            size: 40,
            on_send: None,
            on_deposit: None,
        }
    }

    pub fn bigger(self) -> Self {
        Self { size: 50, ..self }
    }

    pub fn on_send(self, message: Message) -> Self {
        Self {
            on_send: Some(message),
            ..self
        }
    }

    pub fn on_deposit(self, message: Message) -> Self {
        Self {
            on_deposit: Some(message),
            ..self
        }
    }

    pub fn view(self) -> Column<'static, Message> {
        let (balance, pending) = match self.balance {
            Some(balance) => {
                let pending_balance =
                    balance.untrusted_pending + balance.trusted_pending + balance.immature;

                (
                    Text::new(format!("{} sat", format::number(balance.confirmed))),
                    if pending_balance > 0 {
                        Text::new(format!("Pending: +{} sat", format::number(pending_balance)))
                    } else {
                        Text::new("")
                    },
                )
            }
            None => (Text::new("Unavailable"), Text::new("")),
        };

        let btn_size: f32 = self.size as f32 * 3.0 + 30.0;

        let mut send_btn =
            button::border_with_icon(ARROW_UP, "Send").width(Length::Fixed(btn_size));
        let mut deposit_btn =
            button::border_with_icon(ARROW_DOWN, "Receive").width(Length::Fixed(btn_size));

        if let Some(on_send) = self.on_send {
            send_btn = send_btn.on_press(on_send);
        }

        if let Some(on_deposit) = self.on_deposit {
            deposit_btn = deposit_btn.on_press(on_deposit);
        }

        Column::new()
            .push(
                Column::new()
                    .push(balance.size(self.size).view())
                    .push(pending.size(self.size / 2).extra_light().view())
                    .spacing(10)
                    .width(Length::Fill)
                    .align_items(Alignment::Center),
            )
            .push(Row::new().push(send_btn).push(deposit_btn).spacing(10))
            .spacing(20)
            .width(Length::Fill)
            .align_items(Alignment::Center)
    }
}
