// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use iced::widget::{button, Button, Container, Row};
use iced::{theme, Alignment, Background, Length, Theme, Vector};

use super::{Icon, Text};
use crate::theme::color::{GREY, ORANGE, TRANSPARENT, WHITE};

pub fn primary<'a, T: 'a>(t: &'static str) -> Button<'a, T> {
    Button::new(content(None, t)).style(PrimaryButtonStyle.into())
}

pub fn border<'a, T: 'a>(t: &'static str) -> Button<'a, T> {
    Button::new(content(None, t)).style(BorderButtonStyle.into())
}

pub fn primary_with_icon<'a, T: 'a>(icon: char, t: &'static str) -> Button<'a, T> {
    Button::new(content(Some(icon), t)).style(PrimaryButtonStyle.into())
}

pub fn border_with_icon<'a, T: 'a>(icon: char, t: &'static str) -> Button<'a, T> {
    Button::new(content(Some(icon), t)).style(BorderButtonStyle.into())
}

pub fn secondary<'a, T: 'a>(t: &'static str) -> Button<'a, T> {
    Button::new(content(None, t)).style(SecondaryButtonStyle.into())
}

fn content<'a, T: 'a>(icon: Option<char>, t: &'static str) -> Container<'a, T> {
    match icon {
        None => Container::new(Text::new(t).view())
            .width(Length::Fill)
            .center_x()
            .padding(5),
        Some(icon) => Container::new(
            Row::new()
                .push(Icon::new(icon).view())
                .push(Text::new(t).view())
                .spacing(10)
                .width(Length::Fill)
                .align_items(Alignment::Center),
        )
        .width(Length::Fill)
        .center_x()
        .padding(5),
    }
}

pub struct PrimaryButtonStyle;

impl button::StyleSheet for PrimaryButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(ORANGE)),
            border_radius: 10.0,
            border_width: 0.0,
            border_color: TRANSPARENT,
            text_color: WHITE,
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

pub struct SecondaryButtonStyle;

impl button::StyleSheet for SecondaryButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::default(),
            background: Some(Background::Color(GREY)),
            border_radius: 10.0,
            border_width: 0.0,
            border_color: TRANSPARENT,
            text_color: WHITE,
        }
    }
}

impl From<SecondaryButtonStyle> for theme::Button {
    fn from(style: SecondaryButtonStyle) -> Self {
        theme::Button::Custom(Box::new(style))
    }
}
