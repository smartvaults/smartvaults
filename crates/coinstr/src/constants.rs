// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

pub const APP_NAME: &str = "Coinstr";
pub const APP_LOGO_MAINNET: &[u8] = include_bytes!("../static/img/coinstr.svg");
pub const APP_LOGO_TESTNET: &[u8] = include_bytes!("../static/img/coinstr-testnet.svg");
pub const APP_LOGO_SIGNET: &[u8] = include_bytes!("../static/img/coinstr-signet.svg");
pub const APP_LOGO_REGTEST: &[u8] = include_bytes!("../static/img/coinstr-regtest.svg");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

pub const BIGGER_FONT_SIZE: u8 = 19;
pub const BIG_FONT_SIZE: u8 = 17;
pub const DEFAULT_FONT_SIZE: u8 = 15;
pub const SMALL_FONT_SIZE: u8 = 13;
pub const SMALLER_FONT_SIZE: u8 = 11;

pub const DEFAULT_ICON_SIZE: u8 = 20;
