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

pub fn danger_with_icon<S, T>(icon: char, t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(Some(icon), t)).style(DangerButtonStyle.into())
}

pub fn danger_border<S, T>(t: S) -> Button<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    Button::new(content(None, t)).style(DangerBorderButtonStyle.into())
}

pub fn danger_border_only_icon<T>(icon: char) -> Button<'static, T>
where
    T: Clone + 'static,
{
    Button::new(content(Some(icon), "")).style(DangerBorderButtonStyle.into())
}

pub fn transparent_only_icon<T>(icon: char, color: Option<Color>) -> Button<'static, T>
where
    T: Clone + 'static,
{
    Button::new(content(Some(icon), "")).style(TransparentButtonStyle::new(color).into())
}

fn content<S, T>(icon: Option<char>, t: S) -> Container<'static, T>
where
    S: Into<String>,
    T: Clone + 'static,
{
    match icon {
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
        None => Container::new(Text::new(t).view())
            .width(Length::Fill)
            .center_x()
            .padding(5),
    }
}

pub struct PrimaryButtonStyle;

impl button::StyleSheet for PrimaryButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let palette = style.palette();
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(palette.primary)),
            border_radius: 10.0,
            border_width: 0.0,
            border_color: TRANSPARENT,
            text_color: palette.text,
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

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let palette = style.palette();
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(TRANSPARENT)),
            border_radius: 10.0,
            border_width: 1.0,
            border_color: palette.primary,
            text_color: palette.primary,
        }
    }
}

impl From<BorderButtonStyle> for theme::Button {
    fn from(style: BorderButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}

pub struct DangerButtonStyle;

impl button::StyleSheet for DangerButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let palette = style.palette();
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(palette.danger)),
            border_radius: 10.0,
            border_width: 0.0,
            border_color: TRANSPARENT,
            text_color: palette.text,
        }
    }
}

impl From<DangerButtonStyle> for theme::Button {
    fn from(style: DangerButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}

pub struct DangerBorderButtonStyle;

impl button::StyleSheet for DangerBorderButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let palette = style.palette();
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(TRANSPARENT)),
            border_radius: 10.0,
            border_width: 1.0,
            border_color: palette.danger,
            text_color: palette.danger,
        }
    }
}

impl From<DangerBorderButtonStyle> for theme::Button {
    fn from(style: DangerBorderButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}

pub struct TransparentButtonStyle {
    text_color: Option<Color>,
}

impl TransparentButtonStyle {
    pub fn new(text_color: Option<Color>) -> Self {
        Self { text_color }
    }
}

impl button::StyleSheet for TransparentButtonStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> button::Appearance {
        let palette = style.palette();
        button::Appearance {
            background: Some(Background::Color(TRANSPARENT)),
            border_color: TRANSPARENT,
            text_color: self.text_color.unwrap_or(palette.text),
            ..Default::default()
        }
    }
}

impl From<TransparentButtonStyle> for theme::Button {
    fn from(style: TransparentButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}
