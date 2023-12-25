// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use iced::font::{Family, Stretch, Style, Weight};
use iced::Font;

pub const BOOTSTRAP_ICONS_BYTES: &[u8] = include_bytes!("../../static/icon/bootstrap-icons.otf");

pub const ROBOTO_MONO_REGULAR_BYTES: &[u8] =
    include_bytes!("../../static/font/RobotoMono-Regular.ttf");
pub const ROBOTO_MONO_LIGHT_BYTES: &[u8] = include_bytes!("../../static/font/RobotoMono-Light.ttf");
pub const ROBOTO_MONO_BOLD_BYTES: &[u8] = include_bytes!("../../static/font/RobotoMono-Bold.ttf");

pub const ICON_FONT: Font = Font::with_name("bootstrap-icons");

pub const REGULAR: Font = Font {
    family: Family::Name("Roboto Mono"),
    weight: Weight::Normal,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

pub const EXTRA_LIGHT: Font = Font {
    family: Family::Name("Roboto Mono"),
    weight: Weight::Light,
    stretch: Stretch::Normal,
    style: Style::Normal,
};

pub const BOLD: Font = Font {
    family: Family::Name("Roboto Mono"),
    weight: Weight::Bold,
    stretch: Stretch::Normal,
    style: Style::Normal,
};
