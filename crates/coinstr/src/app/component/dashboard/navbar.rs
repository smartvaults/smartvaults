// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bips::bip32::Bip32;
use coinstr_sdk::util::format;
use iced::widget::Row;
use iced::{Alignment, Color, Length};

use crate::app::component::breadcrumb::Breadcrumb;
use crate::app::{Context, Message, Stage};
use crate::component::{rule, Button, ButtonStyle, Icon, Text};
use crate::theme::color::{DARK_RED, RED};
use crate::theme::icon::{BELL, BOX, EYE, EYE_SLASH, FINGERPRINT, PERSON_CIRCLE};

#[derive(Clone, Default)]
pub struct Navbar;

impl Navbar {
    pub fn new() -> Self {
        Self
    }

    pub fn view<'a>(&self, ctx: &Context) -> Row<'a, Message> {
        let color: Option<Color> = match ctx.client.count_unseen_notifications() {
            Ok(count) => {
                if count > 0 {
                    Some(RED)
                } else {
                    None
                }
            }
            Err(e) => {
                tracing::error!("Impossible to count unseen notifications: {e}");
                None
            }
        };

        // Identity
        let fingerprint = match ctx.client.keychain().seed.fingerprint(ctx.client.network()) {
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
            .push(rule::vertical())
            .push(
                Row::new()
                    .push(Icon::new(BOX).view())
                    .push(Text::new(format::number(ctx.client.block_height() as u64)).view())
                    .align_items(Alignment::Center)
                    .padding(10)
                    .spacing(10),
            )
            .push(rule::vertical())
            .push(
                Row::new()
                    .push(Icon::new(FINGERPRINT).view())
                    .push(fingerprint.view())
                    .align_items(Alignment::Center)
                    .padding(10)
                    .spacing(10),
            )
            .push(rule::vertical())
            .push(
                Button::new()
                    .icon(BELL)
                    .style(ButtonStyle::Transparent { text_color: color })
                    .on_press(Message::View(Stage::Notifications))
                    .width(Length::Fixed(40.0))
                    .view(),
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
