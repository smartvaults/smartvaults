// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Column, Row};
use iced::{Alignment, Length};

use crate::app::{Context, Message, Stage};
use crate::component::button;
use crate::theme::color::RED;
use crate::theme::icon::{BELL, BELL_FILL};

#[derive(Clone, Default)]
pub struct Navbar;

impl Navbar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<'a>(&self, ctx: &Context) -> Row<'a, Message> {
        let mut button = button::transparent_only_icon(BELL, None);
        match ctx.client.db.count_unseen_notifications() {
            Ok(count) => {
                if count > 0 {
                    button = button::transparent_only_icon(BELL_FILL, Some(RED));
                }
            }
            Err(e) => {
                log::error!("Impossible to count unseen notifications: {e}");
            }
        };

        Row::new()
            .push(Column::new().width(Length::Fill))
            .push(
                button
                    .on_press(Message::View(Stage::Notifications))
                    .width(Length::Fixed(40.0)),
            )
            .spacing(10)
            .padding(10)
            .height(Length::Fixed(60.0))
            .align_items(Alignment::Center)
    }
}
