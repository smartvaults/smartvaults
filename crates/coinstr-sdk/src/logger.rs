// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::env;
use std::path::Path;

use bdk::bitcoin::Network;
use fern::{Dispatch, InitError};
use log::LevelFilter;
use nostr_sdk::Timestamp;
use thiserror::Error;

use crate::util::dir;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Dir(#[from] dir::Error),
    #[error(transparent)]
    Log(#[from] log::SetLoggerError),
    #[error(transparent)]
    Logger(#[from] InitError),
}

pub fn init<P>(base_path: P, network: Network) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let path = dir::logs_path(base_path, network)?;
    let mut log_file = path.join(Timestamp::now().as_u64().to_string());
    log_file.set_extension("log");

    let mut dispatcher = Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Timestamp::now().to_human_datetime(),
                record.level(),
                record.target(),
                message
            ))
        })
        // Default filter
        .level(LevelFilter::Debug);

    if !cfg!(debug_assertions) {
        dispatcher = dispatcher
            // Crates filters
            .level_for("bdk", LevelFilter::Info)
            .level_for("rustls", LevelFilter::Off)
    }

    if let Ok(stdout) = env::var("STDOUT_LOG") {
        if stdout == "true" {
            dispatcher = dispatcher.chain(std::io::stdout());
        }
    }

    dispatcher.chain(fern::log_file(log_file)?).apply()?;

    Ok(())
}
