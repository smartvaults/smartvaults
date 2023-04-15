// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

#![allow(dead_code)]

use iced::theme::{Palette, Theme as NativeTheme};

pub mod color;
pub mod font;
pub mod icon;

use self::color::{BLACK, GREEN, NEUTRAL, ORANGE, RED, WHITE};

const LIGHT: Palette = Palette {
    background: WHITE,
    text: BLACK,
    primary: ORANGE,
    success: GREEN,
    danger: RED,
};

const DARK: Palette = Palette {
    background: BLACK,
    text: NEUTRAL,
    primary: ORANGE,
    success: GREEN,
    danger: RED,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub fn palette(&self) -> Palette {
        match self {
            Self::Light => LIGHT,
            Self::Dark => DARK,
        }
    }
}

impl From<Theme> for NativeTheme {
    fn from(theme: Theme) -> Self {
        Self::custom(theme.palette())
    }
}
