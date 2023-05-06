// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::Font;

pub const REGULAR_BYTES: &[u8] = include_bytes!("../../static/font/RobotoMono-Regular.ttf");

pub const REGULAR: Font = Font::External {
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
};
