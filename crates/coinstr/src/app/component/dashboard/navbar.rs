// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Column, Row};
use iced::{Alignment, Color, Length};

use crate::app::{Context, Message, Stage};
use crate::component::button;
use crate::theme::color::RED;
use crate::theme::icon::BELL;

#[derive(Clone, Default)]
pub struct Navbar;

impl Navbar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<'a>(&self, ctx: &Context) -> Row<'a, Message> {
        let color: Option<Color> = match ctx.client.db.count_unseen_notifications() {
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
            .push(Column::new().width(Length::Fill))
            .push(
                button::transparent_only_icon(BELL, color)
                    .on_press(Message::View(Stage::Notifications))
                    .width(Length::Fixed(40.0)),
            )
            .spacing(10)
            .padding(10)
            .height(Length::Fixed(60.0))
            .align_items(Alignment::Center)
    }
}
