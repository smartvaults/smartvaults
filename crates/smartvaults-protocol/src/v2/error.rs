// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Error

use smartvaults_core::bitcoin::psbt::PsbtParseError;
use smartvaults_core::bitcoin::{address, consensus};
use smartvaults_core::{miniscript, policy, secp256k1};
use thiserror::Error;

use super::core::{CryptoError, SchemaError};
use super::network;

/// Protocol V2 Error
#[allow(missing_docs)]
#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    BitcoinConsensus(#[from] consensus::encode::Error),
    #[error(transparent)]
    Secp256k1(#[from] secp256k1::Error),
    #[error(transparent)]
    Policy(#[from] policy::Error),
    #[error(transparent)]
    Network(#[from] network::Error),
    #[error(transparent)]
    Address(#[from] address::Error),
    #[error(transparent)]
    Psbt(#[from] PsbtParseError),
    #[error(transparent)]
    Miniscript(#[from] miniscript::Error),
    #[error(transparent)]
    Crypto(#[from] CryptoError),
    #[error(transparent)]
    Schema(#[from] SchemaError),
    #[error(transparent)]
    Proto(#[from] prost::DecodeError),
    #[error(transparent)]
    Keys(#[from] nostr::key::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr::event::builder::Error),
    #[error("{0} not found")]
    NotFound(String),
}