// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;

pub type Result<T, E = FFIError> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum FFIError {
    Generic { err: String },
}

impl fmt::Display for FFIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic { err } => write!(f, "{err}"),
        }
    }
}

impl From<std::io::Error> for FFIError {
    fn from(e: std::io::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_core::client::Error> for FFIError {
    fn from(e: coinstr_core::client::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_core::nostr_sdk::event::id::Error> for FFIError {
    fn from(e: coinstr_core::nostr_sdk::event::id::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}