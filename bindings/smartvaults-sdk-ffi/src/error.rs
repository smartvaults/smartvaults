// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::fmt;

use smartvaults_sdk::core::miniscript::descriptor::DescriptorKeyParseError;
use uniffi::Error;

pub type Result<T, E = SmartVaultsError> = std::result::Result<T, E>;

#[derive(Debug, Error)]
#[uniffi(flat_error)]
pub enum SmartVaultsError {
    Generic(String),
}

impl std::error::Error for SmartVaultsError {}

impl fmt::Display for SmartVaultsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Generic(e) => write!(f, "{e}"),
        }
    }
}

impl From<smartvaults_sdk::logger::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::logger::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<std::io::Error> for SmartVaultsError {
    fn from(e: std::io::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::config::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::config::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::nostr::types::url::ParseError> for SmartVaultsError {
    fn from(e: smartvaults_sdk::nostr::types::url::ParseError) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::nostr::key::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::nostr::key::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::nostr::nips::nip19::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::nostr::nips::nip19::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::nostr::nips::nip46::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::nostr::nips::nip46::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::nostr::types::metadata::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::nostr::types::metadata::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::bitcoin::secp256k1::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::bitcoin::secp256k1::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::bitcoin::psbt::PsbtParseError> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::bitcoin::psbt::PsbtParseError) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::nostr::event::id::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::nostr::event::id::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::bips::bip39::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::bips::bip39::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::bitcoin::address::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::bitcoin::address::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::bitcoin::hashes::hex::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::bitcoin::hashes::hex::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::util::dir::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::util::dir::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::policy::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::policy::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::signer::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::signer::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<DescriptorKeyParseError> for SmartVaultsError {
    fn from(e: DescriptorKeyParseError) -> SmartVaultsError {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::bitcoin::absolute::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::bitcoin::absolute::Error) -> SmartVaultsError {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::core::miniscript::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::core::miniscript::Error) -> Self {
        Self::Generic(e.to_string())
    }
}

impl From<nostr_ffi::NostrError> for SmartVaultsError {
    fn from(e: nostr_ffi::NostrError) -> SmartVaultsError {
        Self::Generic(e.to_string())
    }
}

impl From<smartvaults_sdk::protocol::v1::key_agent::signer::Error> for SmartVaultsError {
    fn from(e: smartvaults_sdk::protocol::v1::key_agent::signer::Error) -> SmartVaultsError {
        Self::Generic(e.to_string())
    }
}
