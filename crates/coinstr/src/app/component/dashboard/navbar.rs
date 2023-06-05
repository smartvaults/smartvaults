// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{Column, Row};
use iced::{Alignment, Length};

use crate::app::{Context, Message};
use crate::component::button;
// use crate::theme::color::RED;
use crate::theme::icon::BELL;

#[derive(Clone, Default)]
pub struct Navbar;

impl Navbar {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view<'a>(&self, _ctx: &Context) -> Row<'a, Message> {
        Row::new()
            .push(Column::new().width(Length::Fill))
            .push(button::transparent_only_icon(BELL, None).width(Length::Fixed(40.0)))
            .spacing(10)
            .padding(10)
            .height(Length::Fixed(60.0))
            .align_items(Alignment::Center)
    }
}
