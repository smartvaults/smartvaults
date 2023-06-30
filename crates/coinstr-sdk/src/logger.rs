// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::env;
use std::path::Path;

use fern::{Dispatch, InitError};
use log::LevelFilter;
use nostr_sdk::Timestamp;

pub(crate) fn init<P>(path: P) -> Result<(), InitError>
where
    P: AsRef<Path>,
{
    let mut log_file = path.as_ref().join(Timestamp::now().as_u64().to_string());
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
