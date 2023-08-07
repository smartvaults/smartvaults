// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bitcoin::Network;
use coinstr_sdk::logger;

use crate::error::Result;

pub fn init_logger(base_path: String, network: Network) -> Result<()> {
    Ok(logger::init(base_path, network, true)?)
}
