// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::env;
use std::path::{Path, PathBuf};

use nostr_sdk::Timestamp;
use smartvaults_core::bitcoin::Network;
use thiserror::Error;
use tracing::Level;
use tracing_subscriber::filter::Targets;
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::reload::Layer as ReloadLayer;
use tracing_subscriber::util::{SubscriberInitExt, TryInitError};
use tracing_subscriber::{fmt, Layer};

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

fn targets_filter() -> Targets {
    let trace: bool = env::var("SMARTVAULTS_TRACE") == Ok(String::from("true"));
    Targets::new()
        .with_default(Level::WARN)
        .with_target("bdk", Level::INFO)
        .with_target("bdk::database::sqlite", Level::WARN)
        .with_target("keechain_core", Level::INFO)
        .with_target("nostr", Level::DEBUG)
        .with_target(
            "nostr_database",
            if trace { Level::TRACE } else { Level::DEBUG },
        )
        .with_target("nostr_sqlite", Level::INFO)
        .with_target("nostr_sdk", Level::DEBUG)
        .with_target(
            "smartvaults_core",
            if trace { Level::TRACE } else { Level::DEBUG },
        )
        .with_target(
            "smartvaults_protocol",
            if trace { Level::TRACE } else { Level::DEBUG },
        )
        .with_target(
            "smartvaults_sdk",
            if trace { Level::TRACE } else { Level::DEBUG },
        )
        .with_target("smartvaults_desktop", Level::DEBUG)
        .with_target("smartvaults_sdk_ffi", Level::INFO)
}

//#[cfg(all(not(target_os = "android"), not(target_os = "ios")))]
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
    let file_log = fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_file(false);
    let (file_log, ..) = ReloadLayer::new(file_log);

    let targets_filter = targets_filter();

    if stdout {
        let stdout_log = fmt::layer()
            .with_ansi(true)
            .with_file(false)
            .with_span_events(FmtSpan::CLOSE);
        tracing_subscriber::registry()
            .with(stdout_log.and_then(file_log).with_filter(targets_filter))
            .try_init()?;
    } else {
        tracing_subscriber::registry()
            .with(file_log.with_filter(targets_filter))
            .try_init()?;
    };

    Ok(())
}

//#[cfg(any(target_os = "android", target_os = "ios"))]
pub fn init_mobile() {
    #[cfg(target_os = "android")]
    let layer = fmt::layer().with_writer(paranoid_android::AndroidLogMakeWriter::new(
        "io.smartvaults.sdk".to_owned(),
    ));

    #[cfg(not(target_os = "android"))]
    let layer = fmt::layer();

    match tracing_subscriber::registry()
        .with(
            layer
                .with_ansi(false)
                .with_file(false)
                .with_span_events(FmtSpan::CLOSE)
                .with_filter(targets_filter()),
        )
        .try_init()
    {
        Ok(_) => tracing::info!("Logger initialized"),
        Err(e) => eprintln!("Impossible to init logger: {e}"),
    }
}
