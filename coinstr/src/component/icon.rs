// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::alignment::Horizontal;
use iced::widget::Text;
use iced::{Color, Font, Length};

const ICONS: Font = Font::External {
    name: "Icons",
    bytes: include_bytes!("../../static/icon/bootstrap-icons.otf"),
};

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
            size: 20,
            width: Length::Fixed(20.0),
            color: None,
        }
    }

    /* pub fn size(self, size: u16) -> Self {
        Self { size, ..self }
    }

    pub fn width(self, width: Length) -> Self {
        Self { width, ..self }
    }

    pub fn color(self, color: Color) -> Self {
        Self {
            color: Some(color),
            ..self
        }
    } */

    pub fn view(self) -> Text<'static> {
        let mut icon = Text::new(self.unicode.to_string())
            .font(ICONS)
            .width(self.width)
            .horizontal_alignment(Horizontal::Center)
            .size(self.size);

        if let Some(color) = self.color {
            icon = icon.style(color);
        }

        icon
    }
}
