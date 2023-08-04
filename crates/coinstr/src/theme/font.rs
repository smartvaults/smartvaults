// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::font::{Family, Stretch, Weight};
use iced::Font;

//pub const REGULAR_BYTES: &[u8] = include_bytes!("../../static/font/RobotoMono-Regular.ttf");

pub const REGULAR: Font = Font {
    family: Family::SansSerif,
    weight: Weight::Normal,
    monospaced: true,
    stretch: Stretch::Normal,
};

pub const EXTRA_LIGHT: Font = Font {
    family: Family::SansSerif,
    weight: Weight::ExtraLight,
    monospaced: true,
    stretch: Stretch::Normal,
};

pub const BOLD: Font = Font {
    family: Family::SansSerif,
    weight: Weight::Bold,
    monospaced: true,
    stretch: Stretch::Normal,
};

/* pub const REGULAR: Font = Font::External {
    name: "Regular",
    bytes: REGULAR_BYTES,
};

pub const EXTRA_LIGHT: Font = Font::External {
    name: "ExtraLight",
    bytes: include_bytes!("../../static/font/RobotoMono-ExtraLight.ttf"),
};

pub const BOLD: Font = Font::External {
    name: "Bold",
    bytes: include_bytes!("../../static/font/RobotoMono-Bold.ttf"),
}; */
