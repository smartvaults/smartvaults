// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::fmt;

use smartvaults_sdk::core::miniscript::descriptor::DescriptorKeyParseError;

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

impl From<smartvaults_sdk::logger::Error> for FFIError {
    fn from(e: smartvaults_sdk::logger::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<std::io::Error> for FFIError {
    fn from(e: std::io::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::client::Error> for FFIError {
    fn from(e: smartvaults_sdk::client::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::config::Error> for FFIError {
    fn from(e: smartvaults_sdk::config::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::nostr::url::ParseError> for FFIError {
    fn from(e: smartvaults_sdk::nostr::url::ParseError) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::nostr::key::Error> for FFIError {
    fn from(e: smartvaults_sdk::nostr::key::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::nostr::nips::nip19::Error> for FFIError {
    fn from(e: smartvaults_sdk::nostr::nips::nip19::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::nostr::nips::nip46::Error> for FFIError {
    fn from(e: smartvaults_sdk::nostr::nips::nip46::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::nostr::types::metadata::Error> for FFIError {
    fn from(e: smartvaults_sdk::nostr::types::metadata::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::bdk::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::bdk::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::bitcoin::secp256k1::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::bitcoin::secp256k1::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::bitcoin::psbt::PsbtParseError> for FFIError {
    fn from(e: smartvaults_sdk::core::bitcoin::psbt::PsbtParseError) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::nostr::event::id::Error> for FFIError {
    fn from(e: smartvaults_sdk::nostr::event::id::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::bips::bip39::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::bips::bip39::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::bitcoin::address::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::bitcoin::address::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::bitcoin::hashes::hex::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::bitcoin::hashes::hex::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::util::dir::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::util::dir::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::policy::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::policy::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::signer::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::signer::Error) -> Self {
        Self::Generic { err: e.to_string() }
    }
}

impl From<DescriptorKeyParseError> for FFIError {
    fn from(e: DescriptorKeyParseError) -> FFIError {
        Self::Generic { err: e.to_string() }
    }
}

impl From<smartvaults_sdk::core::bitcoin::absolute::Error> for FFIError {
    fn from(e: smartvaults_sdk::core::bitcoin::absolute::Error) -> FFIError {
        Self::Generic { err: e.to_string() }
    }
}

impl From<nostr_sdk_ffi::NostrError> for FFIError {
    fn from(e: nostr_sdk_ffi::NostrError) -> FFIError {
        Self::Generic { err: e.to_string() }
    }
}