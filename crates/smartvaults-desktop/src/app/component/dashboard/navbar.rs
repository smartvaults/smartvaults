// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::widget::Row;
use iced::{Alignment, Length};
use smartvaults_sdk::core::bips::bip32::Bip32;
use smartvaults_sdk::core::SECP256K1;
use smartvaults_sdk::util::format;

use crate::app::component::breadcrumb::Breadcrumb;
use crate::app::{Context, Message, Stage};
use crate::component::{rule, Button, ButtonStyle, Icon, Text};
use crate::theme::color::DARK_RED;
use crate::theme::icon::{BOX, EYE, EYE_SLASH, FINGERPRINT, PERSON_CIRCLE};

#[derive(Clone, Default)]
pub struct Navbar;

impl Navbar {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&self, ctx: &Context) -> Row<'a, Message> {
        // Identity
        let fingerprint = match ctx
            .client
            .keychain()
            .seed
            .fingerprint(ctx.client.network(), &SECP256K1)
        {
            Ok(fingerprint) => Text::new(fingerprint.to_string()),
            Err(_) => Text::new("error").color(DARK_RED),
        };

        Row::new()
            .push(
                Row::new()
                    .push(Breadcrumb::new(ctx.breadcrumb.clone()).view())
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .spacing(10)
                    .padding(10)
                    .align_items(Alignment::Center),
            )
            .push(
                Row::new()
                    .push(rule::vertical())
                    .height(Length::Fixed(40.0)),
            )
            .push(
                Row::new()
                    .push(Icon::new(BOX))
                    .push(Text::new(format::number(ctx.client.block_height() as u64)).view())
                    .align_items(Alignment::Center)
                    .padding(10)
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(rule::vertical())
                    .height(Length::Fixed(40.0)),
            )
            .push(
                Row::new()
                    .push(Icon::new(FINGERPRINT))
                    .push(fingerprint.view())
                    .align_items(Alignment::Center)
                    .padding(10)
                    .spacing(10),
            )
            .push(
                Row::new()
                    .push(rule::vertical())
                    .height(Length::Fixed(40.0)),
            )
            .push(
                Button::new()
                    .icon(if ctx.hide_balances { EYE_SLASH } else { EYE })
                    .on_press(Message::ToggleHideBalances)
                    .style(ButtonStyle::Transparent { text_color: None })
                    .width(Length::Fixed(40.0))
                    .view(),
            )
            .push(
                Button::new()
                    .icon(PERSON_CIRCLE)
                    .style(ButtonStyle::Transparent { text_color: None })
                    .on_press(Message::View(Stage::Profile))
                    .width(Length::Fixed(40.0))
                    .view(),
            )
            .spacing(10)
            .padding(5)
            .height(Length::Fixed(60.0))
            .align_items(Alignment::Center)
    }
}
