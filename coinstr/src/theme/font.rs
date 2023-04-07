// Copyright (c) 2022-2023 Yuki Kishimoto
// Distributed under the MIT software license

use iced::Font;

pub const REGULAR_BYTES: &[u8] = include_bytes!("../../static/font/RobotoMono-Regular.ttf");

pub const REGULAR: Font = Font::External {
    name: "Regular",
    bytes: REGULAR_BYTES,
};

pub const BOLD: Font = Font::External {
    name: "Bold",
    bytes: include_bytes!("../../static/font/RobotoMono-Bold.ttf"),
};
