// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use iced::widget::Text as NativeText;
use iced::{Color, Font};

use crate::theme::font::{BOLD, REGULAR};

pub struct Text {
    content: String,
    size: u16,
    color: Option<Color>,
    font: Font,
}

impl Text {
    pub fn new<S>(content: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            content: content.into(),
            size: 20,
            color: None,
            font: REGULAR,
        }
    }

    pub fn size(self, size: u16) -> Self {
        Self { size, ..self }
    }

    pub fn color(self, color: Color) -> Self {
        Self {
            color: Some(color),
            ..self
        }
    }

    pub fn bold(self) -> Self {
        Self { font: BOLD, ..self }
    }

    pub fn view<'a>(self) -> NativeText<'a> {
        let mut text = NativeText::new(self.content)
            .size(self.size)
            .font(self.font);

        if let Some(color) = self.color {
            text = text.style(color);
        }

        text
    }
}
