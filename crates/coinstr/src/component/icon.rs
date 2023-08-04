// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::alignment::Horizontal;
use iced::widget::Text;
use iced::{Color, Length};

use crate::constants::DEFAULT_ICON_SIZE;
use crate::theme::font::ICON_FONT;

pub struct Icon {
    unicode: char,
    size: u16,
    width: Length,
    color: Option<Color>,
}

impl Icon {
    pub fn new(unicode: char) -> Self {
        Self {
            unicode,
            size: DEFAULT_ICON_SIZE as u16,
            width: Length::Fixed(DEFAULT_ICON_SIZE as f32),
            color: None,
        }
    }

    /* pub fn size(self, size: u16) -> Self {
        Self { size, ..self }
    } */

    pub fn width(self, width: Length) -> Self {
        Self { width, ..self }
    }

    pub fn color(self, color: Color) -> Self {
        Self {
            color: Some(color),
            ..self
        }
    }

    pub fn view(self) -> Text<'static> {
        let mut icon = Text::new(self.unicode.to_string())
            .font(ICON_FONT)
            .width(self.width)
            .horizontal_alignment(Horizontal::Center)
            .size(self.size);

        if let Some(color) = self.color {
            icon = icon.style(color);
        }

        icon
    }
}
