// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{button, Button, Container, Row};
use iced::{theme, Alignment, Background, Color, Length, Theme, Vector};

use super::{Icon, Text};
use crate::theme::color::TRANSPARENT;

pub fn primary<S, T>(t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(None, t)).style(ButtonStyle::Primary.into())
}

pub fn border<S, T>(t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(None, t)).style(ButtonStyle::Bordered.into())
}

pub fn primary_with_icon<S, T>(icon: char, t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(Some(icon), t)).style(ButtonStyle::Primary.into())
}

pub fn primary_only_icon<T>(icon: char) -> Button<'static, T>
where
    T: Clone + 'static,
{
    Button::new(content(Some(icon), "")).style(ButtonStyle::Primary.into())
}

pub fn border_only_icon<T>(icon: char) -> Button<'static, T>
where
    T: Clone + 'static,
{
    Button::new(content(Some(icon), "")).style(ButtonStyle::Bordered.into())
}

pub fn border_with_icon<S, T>(icon: char, t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(Some(icon), t)).style(ButtonStyle::Bordered.into())
}

pub fn danger_with_icon<S, T>(icon: char, t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(Some(icon), t)).style(ButtonStyle::Danger.into())
}

pub fn danger_border<S, T>(t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(None, t)).style(ButtonStyle::BorderedDanger.into())
}

pub fn danger_border_only_icon<T>(icon: char) -> Button<'static, T>
where
    T: Clone + 'static,
{
    Button::new(content(Some(icon), "")).style(ButtonStyle::BorderedDanger.into())
}

pub fn transparent_only_icon<T>(icon: char, color: Option<Color>) -> Button<'static, T>
where
    T: Clone + 'static,
{
    Button::new(content(Some(icon), ""))
        .style(ButtonStyle::Transparent { text_color: color }.into())
}

fn content<S, T>(icon: Option<char>, t: S) -> Container<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    match icon {
        Some(icon) => {
            let text = t.into();

            let mut icon = Icon::new(icon);

            if text.is_empty() {
                icon = icon.width(Length::Fill);
            }

            let mut row = Row::new()
                .push(icon.view())
                .spacing(10)
                .width(Length::Fill)
                .align_items(Alignment::Center);

            if !text.is_empty() {
                row = row.push(Text::new(text).view());
            }

            Container::new(row)
                .width(Length::Fill)
                .center_x()
                .padding(5)
        }
        None => Container::new(Text::new(t).view())
            .width(Length::Fill)
            .center_x()
            .padding(5),
    }
}

pub enum ButtonStyle {
    Primary,
    Bordered,
    Danger,
    BorderedDanger,
    Transparent { text_color: Option<Color> },
}

impl button::StyleSheet for ButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let palette = style.palette();
        match self {
            Self::Primary => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(palette.primary)),
                border_radius: 10.0,
                border_width: 0.0,
                border_color: TRANSPARENT,
                text_color: palette.text,
            },
            Self::Bordered => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(TRANSPARENT)),
                border_radius: 10.0,
                border_width: 1.0,
                border_color: palette.primary,
                text_color: palette.primary,
            },
            Self::Danger => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(palette.danger)),
                border_radius: 10.0,
                border_width: 0.0,
                border_color: TRANSPARENT,
                text_color: palette.text,
            },
            Self::BorderedDanger => button::Appearance {
                shadow_offset: Vector::default(),
                background: Some(Background::Color(TRANSPARENT)),
                border_radius: 10.0,
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
