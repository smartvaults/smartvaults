// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

pub const APP_NAME: &str = "Coinstr";
pub const APP_LOGO_MAINNET: &[u8] = include_bytes!("../static/img/coinstr.svg");
pub const APP_LOGO_TESTNET: &[u8] = include_bytes!("../static/img/coinstr-testnet.svg");
pub const APP_LOGO_SIGNET: &[u8] = include_bytes!("../static/img/coinstr-signet.svg");
pub const APP_LOGO_REGTEST: &[u8] = include_bytes!("../static/img/coinstr-regtest.svg");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
