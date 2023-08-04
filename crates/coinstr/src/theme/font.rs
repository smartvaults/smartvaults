// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use iced::Font;

pub const ICONS_BYTES: &[u8] = include_bytes!("../../static/icon/bootstrap-icons.otf");

pub const REGULAR_BYTES: &[u8] = include_bytes!("../../static/font/RobotoMono-Regular.ttf");
pub const EXTRA_LIGHT_BYTES: &[u8] = include_bytes!("../../static/font/RobotoMono-ExtraLight.ttf");
pub const BOLD_BYTES: &[u8] = include_bytes!("../../static/font/RobotoMono-Bold.ttf");

pub const ICON_FONT: Font = Font::with_name("bootstrap-icons");

pub const REGULAR: Font = Font::with_name("RobotoMono-Regular");
pub const EXTRA_LIGHT: Font = Font::with_name("RobotoMono-ExtraLight");
pub const BOLD: Font = Font::with_name("RobotoMono-Bold");
