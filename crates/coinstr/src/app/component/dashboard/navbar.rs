// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::util::format;
use iced::widget::Row;
use iced::{Alignment, Color, Length};

use crate::app::{Context, Message, Stage};
use crate::component::{button, rule, Icon, Text};
use crate::theme::color::RED;
use crate::theme::icon::{BELL, BOX, EYE, PERSON_CIRCLE};

#[derive(Clone, Default)]
pub struct Navbar;

impl Navbar {
    pub fn new() -> Self {
        Self::default()
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
                log::error!("Impossible to count unseen notifications: {e}");
                None
            }
        };

        Row::new()
            .push(
                Row::new()
                    .push(Text::new("Path > to > screen").extra_light().view())
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
                    .padding(10)
                    .spacing(10),
            )
            .push(rule::vertical())
            .push(
                button::transparent_only_icon(BELL, color)
                    .on_press(Message::View(Stage::Notifications))
                    .width(Length::Fixed(40.0)),
            )
            .push(button::transparent_only_icon(EYE, None).width(Length::Fixed(40.0)))
            .push(
                button::transparent_only_icon(PERSON_CIRCLE, None)
                    .on_press(Message::View(Stage::Profile))
                    .width(Length::Fixed(40.0)),
            )
            .spacing(10)
            .padding(10)
            .height(Length::Fixed(60.0))
            .align_items(Alignment::Center)
    }
}
