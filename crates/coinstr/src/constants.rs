// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//use coinstr_sdk::core::bitcoin::Network;

pub const APP_NAME: &str = "Coinstr";
pub const APP_LOGO: &[u8] = include_bytes!("../static/img/coinstr.svg");
pub const APP_DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");

/* pub const NETWORKS: [Network; 4] = [
    Network::Bitcoin,
    Network::Testnet,
    Network::Signet,
    Network::Regtest,
]; */
