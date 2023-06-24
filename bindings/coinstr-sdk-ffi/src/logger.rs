// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

#![allow(unused_variables)]

use log::LevelFilter;

pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for LevelFilter {
    fn from(value: LogLevel) -> Self {
        match value {
            LogLevel::Trace => Self::Trace,
            LogLevel::Debug => Self::Debug,
            LogLevel::Info => Self::Info,
            LogLevel::Warn => Self::Warn,
            LogLevel::Error => Self::Error,
        }
    }
}

#[cfg(target_os = "android")]
use android_logger::Config;

pub fn init_logger(level: LogLevel) {
    let level: LevelFilter = level.into();

    #[cfg(target_os = "android")]
    android_logger::init_once(Config::default().with_max_level(level));
}
