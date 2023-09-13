// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row};
use iced::{Alignment, Length};
use smartvaults_sdk::core::bdk::wallet::Balance;
use smartvaults_sdk::util::format;

use crate::app::Message;
use crate::component::{Button, ButtonStyle, Text};
use crate::theme::icon::{ARROW_DOWN, ARROW_UP};

pub struct Balances {
    balance: Balance,
    size: u16,
    hide: bool,
    on_send: Option<Message>,
    on_deposit: Option<Message>,
}

impl Balances {
    pub fn new(balance: Balance) -> Self {
        Self {
            balance,
            size: 35,
            hide: false,
            on_send: None,
            on_deposit: None,
        }
    }

    pub fn bigger(self) -> Self {
        Self { size: 45, ..self }
    }

    pub fn hide(self, hide: bool) -> Self {
        Self { hide, ..self }
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
        let (balance, pending) = if self.hide {
            (Text::new("***** sat"), None)
        } else {
            let pending_balance = self.balance.untrusted_pending
                + self.balance.trusted_pending
                + self.balance.immature;

            (
                Text::new(format!("{} sat", format::number(self.balance.confirmed))),
                if pending_balance > 0 {
                    Some(Text::new(format!(
                        "Pending: +{} sat",
                        format::number(pending_balance)
                    )))
                } else {
                    None
                },
            )
        };

        let btn_size: f32 = self.size as f32 * 3.7 + 30.0;

        let mut send_btn = Button::new()
            .icon(ARROW_UP)
            .text("Send")
            .style(ButtonStyle::Bordered)
            .width(Length::Fixed(btn_size));
        let mut deposit_btn = Button::new()
            .icon(ARROW_DOWN)
            .text("Receive")
            .style(ButtonStyle::Bordered)
            .width(Length::Fixed(btn_size));

        if let Some(on_send) = self.on_send {
            send_btn = send_btn.on_press(on_send);
        }

        if let Some(on_deposit) = self.on_deposit {
            deposit_btn = deposit_btn.on_press(on_deposit);
        }

        Column::new()
            .push({
                let mut content = Column::new()
                    .push(balance.size(self.size).view())
                    .spacing(10)
                    .width(Length::Fill)
                    .align_items(Alignment::Center);

                if let Some(pending) = pending {
                    content = content.push(pending.size(self.size / 2).extra_light().view());
                }

                content
            })
            .push(
                Row::new()
                    .push(send_btn.view())
                    .push(deposit_btn.view())
                    .spacing(10),
            )
            .spacing(20)
            .width(Length::Fill)
            .align_items(Alignment::Center)
    }
}
