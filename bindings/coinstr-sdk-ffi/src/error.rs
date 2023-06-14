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

impl From<coinstr_sdk::client::Error> for FFIError {
    fn from(e: coinstr_sdk::client::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_sdk::core::bdk::Error> for FFIError {
    fn from(e: coinstr_sdk::core::bdk::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_sdk::core::bdk::electrum_client::Error> for FFIError {
    fn from(e: coinstr_sdk::core::bdk::electrum_client::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_sdk::core::bitcoin::secp256k1::Error> for FFIError {
    fn from(e: coinstr_sdk::core::bitcoin::secp256k1::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_sdk::core::bitcoin::psbt::PsbtParseError> for FFIError {
    fn from(e: coinstr_sdk::core::bitcoin::psbt::PsbtParseError) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_sdk::nostr::event::id::Error> for FFIError {
    fn from(e: coinstr_sdk::nostr::event::id::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_sdk::core::bips::bip39::Error> for FFIError {
    fn from(e: coinstr_sdk::core::bips::bip39::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_sdk::core::bitcoin::util::address::Error> for FFIError {
    fn from(e: coinstr_sdk::core::bitcoin::util::address::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<coinstr_sdk::core::util::dir::Error> for FFIError {
    fn from(e: coinstr_sdk::core::util::dir::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}
