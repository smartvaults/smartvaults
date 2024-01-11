// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::fmt;
use std::str::FromStr;

use nostr::Keys;
use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::{Address, OutPoint, Txid};
use smartvaults_core::crypto::hash;
use thiserror::Error;

use super::util::{Encryption, Serde};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Keys(#[from] nostr::key::Error),
    #[error("unknown label kind")]
    UnknownLabelKind,
    #[error("unknown label data")]
    UnknownLabelData,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Address,
    Utxo,
    Txid,
}

impl fmt::Display for LabelKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address => write!(f, "address"),
            Self::Utxo => write!(f, "utxo"),
            Self::Txid => write!(f, "txid"),
        }
    }
}

impl FromStr for LabelKind {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "address" => Ok(Self::Address),
            "utxo" => Ok(Self::Utxo),
            "txid" => Ok(Self::Txid),
            _ => Err(Error::UnknownLabelKind),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelData {
    Address(Address<NetworkUnchecked>),
    Utxo(OutPoint),
    Txid(Txid),
}

impl FromStr for LabelData {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match Address::from_str(s) {
            Ok(address) => Ok(Self::Address(address)),
            Err(_) => match OutPoint::from_str(s) {
                Ok(utxo) => Ok(Self::Utxo(utxo)),
                Err(_) => Err(Error::UnknownLabelKind),
            },
        }
    }
}

impl LabelData {
    pub fn generate_identifier(&self, shared_key: &Keys) -> Result<String, Error> {
        let data = match self {
            Self::Address(addr) => addr.clone().assume_checked().to_string(),
            Self::Utxo(utxo) => utxo.to_string(),
            Self::Txid(txid) => txid.to_string(),
        };
        let unhashed_identifier = format!("{}:{}", shared_key.secret_key()?.display_secret(), data);
        let hash = hash::sha256(unhashed_identifier).to_string();
        Ok(hash[..32].to_string())
    }

    pub fn kind(&self) -> LabelKind {
        match self {
            Self::Address(..) => LabelKind::Address,
            Self::Utxo(..) => LabelKind::Utxo,
            Self::Txid(..) => LabelKind::Txid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    data: LabelData,
    text: String,
}

impl Label {
    pub fn new<S>(data: LabelData, text: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            data,
            text: text.into(),
        }
    }

    pub fn address<S>(address: Address<NetworkUnchecked>, text: S) -> Self
    where
        S: Into<String>,
    {
        Self::new(LabelData::Address(address), text)
    }

    pub fn utxo<S>(utxo: OutPoint, text: S) -> Self
    where
        S: Into<String>,
    {
        Self::new(LabelData::Utxo(utxo), text)
    }

    pub fn txid<S>(txid: Txid, text: S) -> Self
    where
        S: Into<String>,
    {
        Self::new(LabelData::Txid(txid), text)
    }

    pub fn kind(&self) -> LabelKind {
        self.data.kind()
    }

    pub fn data(&self) -> LabelData {
        self.data.clone()
    }

    pub fn text(&self) -> String {
        self.text.clone()
    }

    pub fn generate_identifier(&self, shared_key: &Keys) -> Result<String, Error> {
        self.data.generate_identifier(shared_key)
    }
}

impl Serde for Label {}
impl Encryption for Label {}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use nostr::secp256k1::SecretKey;
    use smartvaults_core::bitcoin::Txid;

    use super::*;

    #[test]
    fn test_generate_identifier() {
        let secret_key =
            SecretKey::from_str("151319b71ef19352fea2540b756771ffe8679d5d846ee7eae004829d8a9bf718")
                .unwrap();
        let shared_key = Keys::new(secret_key);

        let txid =
            Txid::from_str("3faa6bff53689b9763ed77fc693831a14030977f0ea79411b1132d27135eb1a9")
                .unwrap();
        let utxo = OutPoint::new(txid, 0);
        assert_eq!(
            LabelData::Utxo(utxo)
                .generate_identifier(&shared_key)
                .unwrap(),
            String::from("2666dc6af5686c709f757a6d31f0f394")
        );

        let address = Address::from_str("bc1qzqhj36c0ctkty36eqdac9q0gv9lrmnanyff0sn").unwrap();
        assert_eq!(
            LabelData::Address(address)
                .generate_identifier(&shared_key)
                .unwrap(),
            String::from("f225b2d56e21560d31ef180f5ff144c2")
        );
    }
}
