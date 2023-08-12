// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::path::{Path, PathBuf};

use bdk::bitcoin::Network;
use nostr_sdk::Timestamp;
use thiserror::Error;
use tracing::metadata::LevelFilter;
use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::Layer as ReloadLayer;
use tracing_subscriber::util::{SubscriberInitExt, TryInitError};
use tracing_subscriber::Layer;

use crate::util::dir;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Dir(#[from] dir::Error),
    #[error(transparent)]
    Logger(#[from] TryInitError),
}

pub fn init<P>(base_path: P, network: Network, stdout: bool) -> Result<(), Error>
where
    P: AsRef<Path>,
{
    let path = dir::logs_path(base_path, network)?;
    let now = Timestamp::now();
    let human_date = now.to_human_datetime();
    let date: Option<&str> = human_date
        .split('T')
        .collect::<Vec<&str>>()
        .first()
        .copied();
    let path: PathBuf = match date {
        Some(date) => {
            let path = path.join(date);
            std::fs::create_dir_all(path.as_path())?;
            path
        }
        None => path,
    };

    let file_appender = tracing_appender::rolling::never(path, format!("{}.log", now.as_u64()));
    let writer = BoxMakeWriter::new(file_appender);
    let file_log = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_file(false);
    let (file_log, ..) = ReloadLayer::new(file_log);

    let target_filter = Targets::new()
        .with_default(Level::WARN)
        .with_target("bdk", Level::INFO)
        .with_target("keechain_core", Level::INFO)
        .with_target("nostr_sdk", Level::DEBUG)
        .with_target("coinstr_core", Level::DEBUG)
        .with_target("coinstr_sdk", Level::DEBUG)
        .with_target("coinstr", Level::DEBUG)
        .with_target("coinstr_sdk_ffi", LevelFilter::OFF);

    if stdout {
        let stdout_log = tracing_subscriber::fmt::layer()
            .with_ansi(!cfg!(any(target_os = "android", target_os = "ios")))
            .with_file(false);
        tracing_subscriber::registry()
            .with(stdout_log.and_then(file_log).with_filter(target_filter))
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(file_log.with_filter(target_filter))
            .try_init()?;
    };

    Ok(())
}
