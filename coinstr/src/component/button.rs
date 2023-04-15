// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::widget::{button, Button, Column, Container, Row};
use iced::{theme, Alignment, Background, Length, Theme, Vector};

use super::{Icon, Text};
use crate::theme::color::{ORANGE, TRANSPARENT};

pub fn primary<S, T>(t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(None, t)).style(PrimaryButtonStyle.into())
}

pub fn border<S, T>(t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(None, t)).style(BorderButtonStyle.into())
}

pub fn primary_with_icon<S, T>(icon: char, t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(Some(icon), t)).style(PrimaryButtonStyle.into())
}

pub fn border_text_below_icon<S, T>(icon: char, t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    let row = Column::new()
        .push(Icon::new(icon).view())
        .push(Text::new(t).view())
        .spacing(10)
        .width(Length::Fill)
        .align_items(Alignment::Center);

    let content = Container::new(row)
        .width(Length::Fill)
        .center_x()
        .padding(5);

    Button::new(content).style(BorderButtonStyle.into())
}

pub fn primary_only_icon<T>(icon: char) -> Button<'static, T>
where
    T: Clone + 'static,
{
    Button::new(content(Some(icon), "")).style(PrimaryButtonStyle.into())
}

pub fn border_only_icon<T>(icon: char) -> Button<'static, T>
where
    T: Clone + 'static,
{
    Button::new(content(Some(icon), "")).style(BorderButtonStyle.into())
}

pub fn border_with_icon<S, T>(icon: char, t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(Some(icon), t)).style(BorderButtonStyle.into())
}

/* pub fn transparent<S, T>(t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(None, t)).style(TransparentButtonStyle.into())
} */

fn content<S, T>(icon: Option<char>, t: S) -> Container<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    match icon {
        None => Container::new(Text::new(t).view())
            .width(Length::Fill)
            .center_x()
            .padding(5),
        Some(icon) => {
            let mut row = Row::new()
                .push(Icon::new(icon).view())
                .spacing(10)
                .width(Length::Fill)
                .align_items(Alignment::Center);

            let text = t.into();

            if !text.is_empty() {
                row = row.push(Text::new(text).view());
            }

            Container::new(row)
                .width(Length::Fill)
                .center_x()
                .padding(5)
        }
    }
}

pub struct PrimaryButtonStyle;

impl button::StyleSheet for PrimaryButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(ORANGE)),
            border_radius: 10.0,
            border_width: 0.0,
            border_color: TRANSPARENT,
            text_color: style.palette().text,
        }
    }
}

impl From<PrimaryButtonStyle> for theme::Button {
    fn from(style: PrimaryButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}

pub struct BorderButtonStyle;

impl button::StyleSheet for BorderButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(TRANSPARENT)),
            border_radius: 10.0,
            border_width: 1.0,
            border_color: ORANGE,
            text_color: ORANGE,
        }
    }
}

impl From<BorderButtonStyle> for theme::Button {
    fn from(style: BorderButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}

pub struct TransparentButtonStyle;

impl button::StyleSheet for TransparentButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        button::Appearance {
            background: Some(Background::Color(TRANSPARENT)),
            border_color: TRANSPARENT,
            text_color: style.palette().text,
            ..Default::default()
        }
    }
}

impl From<TransparentButtonStyle> for theme::Button {
    fn from(style: TransparentButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}
