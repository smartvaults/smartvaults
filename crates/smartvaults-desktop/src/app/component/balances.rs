// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::widget::{Column, Row};
use iced::{Alignment, Length};
use smartvaults_sdk::core::bdk::wallet::Balance;

use crate::app::Message;
use crate::component::{Amount, AmountSign, Button, ButtonStyle};
use crate::theme::color::YELLOW;
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
        let balance = Amount::new(self.balance.confirmed)
            .size(self.size)
            .bold()
            .hidden(self.hide)
            .view();
        let unconfirmed_balance: u64 =
            self.balance.untrusted_pending + self.balance.trusted_pending + self.balance.immature;

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
                    .push(balance)
                    .spacing(10)
                    .width(Length::Fill)
                    .align_items(Alignment::Center);

                if unconfirmed_balance > 0 {
                    content = content.push(
                        Amount::new(unconfirmed_balance)
                            .sign(AmountSign::Positive)
                            .override_color(YELLOW)
                            .size(self.size * 3 / 5)
                            .hidden(self.hide)
                            .view(),
                    );
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
