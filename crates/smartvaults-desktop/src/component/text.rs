// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use iced::alignment::Horizontal;
use iced::widget::{Button, Text as NativeText};
use iced::{Color, Element, Font, Length};

use super::button::ButtonStyle;
use crate::constants::{
    BIGGER_FONT_SIZE, BIG_FONT_SIZE, DEFAULT_FONT_SIZE, SMALLER_FONT_SIZE, SMALL_FONT_SIZE,
};
use crate::theme::font::{BOLD, EXTRA_LIGHT, REGULAR};

pub struct Text<Message> {
    content: String,
    size: u16,
    color: Option<Color>,
    font: Font,
    width: Option<Length>,
    horizontal_alignment: Option<Horizontal>,
    on_press: Option<Message>,
}

impl<Message> Text<Message>
where
    Message: Clone + 'static,
{
    pub fn new<S>(content: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            content: content.into(),
            size: DEFAULT_FONT_SIZE as u16,
            color: None,
            font: REGULAR,
            width: None,
            horizontal_alignment: None,
            on_press: None,
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

    pub fn color_maybe(mut self, color: Option<Color>) -> Self {
        self.color = color;
        self
    }

    pub fn extra_light(self) -> Self {
        Self {
            font: EXTRA_LIGHT,
            ..self
        }
    }

    pub fn bold(self) -> Self {
        Self { font: BOLD, ..self }
    }

    pub fn small(self) -> Self {
        Self {
            size: SMALL_FONT_SIZE,
            ..self
        }
    }

    pub fn smaller(self) -> Self {
        Self {
            size: SMALLER_FONT_SIZE,
            ..self
        }
    }

    pub fn big(self) -> Self {
        Self {
            size: BIG_FONT_SIZE,
            ..self
        }
    }

    #[allow(dead_code)]
    pub fn bigger(self) -> Self {
        Self {
            size: BIGGER_FONT_SIZE,
            ..self
        }
    }

    pub fn width(self, length: Length) -> Self {
        Self {
            width: Some(length),
            ..self
        }
    }

    pub fn horizontal_alignment(self, alignment: Horizontal) -> Self {
        Self {
            horizontal_alignment: Some(alignment),
            ..self
        }
    }

    pub fn on_press(self, message: Message) -> Self {
        Self {
            on_press: Some(message),
            ..self
        }
    }

    pub fn view(self) -> Element<'static, Message> {
        let mut text = NativeText::new(self.content)
            .size(self.size)
            .font(self.font);

        if let Some(color) = self.color {
            text = text.style(color);
        }

        if let Some(length) = self.width {
            text = text.width(length);
        }

        if let Some(alignment) = self.horizontal_alignment {
            text = text.horizontal_alignment(alignment);
        }

        if let Some(on_press) = self.on_press {
            Button::new(text)
                .on_press(on_press)
                .padding(0)
                .style(ButtonStyle::Transparent { text_color: None })
                .into()
        } else {
            text.into()
        }
    }
}
