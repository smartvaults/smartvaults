// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::logger;

use crate::error::Result;
use crate::Network;

pub fn init_logger(base_path: String, network: Network) -> Result<()> {
    Ok(logger::init(base_path, network.into(), true)?)
}
