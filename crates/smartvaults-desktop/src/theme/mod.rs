// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

#![allow(dead_code)]

use iced::theme::{Palette, Theme as NativeTheme};

pub mod color;
pub mod font;
pub mod icon;

use self::color::{BLACK, BLUE, GREEN, NEUTRAL, ORANGE, PURPLE, RED};

const MAINNET: Palette = Palette {
    background: BLACK,
    text: NEUTRAL,
    primary: ORANGE,
    success: GREEN,
    danger: RED,
};

const TESTNET: Palette = Palette {
    background: BLACK,
    text: NEUTRAL,
    primary: GREEN,
    success: GREEN,
    danger: RED,
};

const SIGNET: Palette = Palette {
    background: BLACK,
    text: NEUTRAL,
    primary: PURPLE,
    success: GREEN,
    danger: RED,
};

const REGTEST: Palette = Palette {
    background: BLACK,
    text: NEUTRAL,
    primary: BLUE,
    success: GREEN,
    danger: RED,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Theme {
    #[default]
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

impl Theme {
    pub fn palette(&self) -> Palette {
        match self {
            Self::Mainnet => MAINNET,
            Self::Testnet => TESTNET,
            Self::Signet => SIGNET,
            Self::Regtest => REGTEST,
        }
    }
}

impl From<Theme> for NativeTheme {
    fn from(theme: Theme) -> Self {
        Self::custom(theme.palette())
    }
}
