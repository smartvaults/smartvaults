// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{self, button, Container, Row};
use iced::{theme, Alignment, Background, BorderRadius, Color, Length, Theme, Vector};

use super::{Icon, Text};
use crate::theme::color::TRANSPARENT;

pub struct Button<Message> {
    text: String,
    icon: Option<char>,
    width: Option<Length>,
    style: ButtonStyle,
    on_press: Option<Message>,
    loading: bool,
}

impl<Message> Button<Message>
where
    Message: Clone + 'static,
{
    pub fn new() -> Self {
        Self {
            text: String::new(),
            icon: None,
            width: None,
            style: ButtonStyle::default(),
            on_press: None,
            loading: false,
        }
    }

    pub fn text<S>(self, text: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            text: text.into(),
            ..self
        }
    }

    pub fn icon(self, icon: char) -> Self {
        Self {
            icon: Some(icon),
            ..self
        }
    }

    pub fn width(self, length: Length) -> Self {
        Self {
            width: Some(length),
            ..self
        }
    }

    pub fn style(self, style: ButtonStyle) -> Self {
        Self { style, ..self }
    }

    pub fn on_press(self, message: Message) -> Self {
        Self {
            on_press: Some(message),
            ..self
        }
    }

    pub fn loading(self, loading: bool) -> Self {
        Self { loading, ..self }
    }

    pub fn view(self) -> widget::Button<'static, Message> {
        let content = match self.icon {
            Some(icon) => {
                let mut icon = Icon::new(icon);

                if self.text.is_empty() {
                    icon = icon.width(Length::Fill);
                }

                let mut row = Row::new()
                    .push(icon.view())
                    .spacing(10)
                    .width(Length::Fill)
                    .align_items(Alignment::Center);

                if !self.text.is_empty() {
                    row = row.push(Text::new(&self.text).view());
                }

                Container::new(row)
                    .width(Length::Fill)
                    .center_x()
                    .padding(5)
            }
            None => Container::new(Text::new(&self.text).view())
                .width(Length::Fill)
                .center_x()
                .padding(5),
        };
        let mut button = widget::Button::new(content);

        if !self.loading {
            if let Some(msg) = self.on_press.clone() {
                button = button.on_press(msg);
            }
        }

        if let Some(width) = self.width {
            button = button.width(width);
        }

        button.style(self.style.into())
    }
}

#[derive(Default, Clone, Copy)]
pub enum ButtonStyle {
    #[default]
    Primary,
    Bordered,
    Danger,
    BorderedDanger,
    Transparent {
        text_color: Option<Color>,
    },
}

impl button::StyleSheet for ButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let palette = style.palette();
        match self {
            Self::Primary => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(palette.primary)),
                border_radius: BorderRadius::from(10.0),
                border_width: 0.0,
                border_color: TRANSPARENT,
                text_color: palette.text,
            },
            Self::Bordered => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(TRANSPARENT)),
                border_radius: BorderRadius::from(10.0),
                border_width: 1.0,
                border_color: palette.primary,
                text_color: palette.primary,
            },
            Self::Danger => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(palette.danger)),
                border_radius: BorderRadius::from(10.0),
                border_width: 0.0,
                border_color: TRANSPARENT,
                text_color: palette.text,
            },
            Self::BorderedDanger => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(TRANSPARENT)),
                border_radius: BorderRadius::from(10.0),
                border_width: 1.0,
                border_color: palette.danger,
                text_color: palette.danger,
            },
            Self::Transparent { text_color } => button::Appearance {
                background: Some(Background::Color(TRANSPARENT)),
                border_color: TRANSPARENT,
                text_color: text_color.unwrap_or(palette.text),
                ..Default::default()
            },
        }
    }
}

impl From<ButtonStyle> for theme::Button {
    fn from(style: ButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}
