// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::bitcoin::{Address, OutPoint};
use coinstr_core::crypto::hash;
use coinstr_core::util::{Encryption, Serde};
use nostr_sdk::Keys;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::util::encryption::EncryptionWithKeys;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LabelKind {
    Address(Address),
    Utxo(OutPoint),
}

impl LabelKind {
    pub fn generate_identifier(&self, shared_keys: &Keys) -> Result<String, Error> {
        let data = match self {
            Self::Address(addr) => addr.to_string(),
            Self::Utxo(utxo) => utxo.to_string(),
        };
        let unhashed_identifier =
            format!("{}:{}", shared_keys.secret_key()?.display_secret(), data);
        let hash = hash::sha256(unhashed_identifier).to_string();
        Ok(hash[..32].to_string())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    kind: LabelKind,
    text: String,
}

impl Serde for Label {}
impl Encryption for Label {}
impl EncryptionWithKeys for Label {}

#[cfg(test)]
mod test {
    use std::str::FromStr;

    use bdk::bitcoin::Txid;
    use nostr_sdk::secp256k1::SecretKey;

    use super::*;

    #[test]
    fn test_generate_identifier() {
        let secret_key =
            SecretKey::from_str("151319b71ef19352fea2540b756771ffe8679d5d846ee7eae004829d8a9bf718")
                .unwrap();
        let shared_keys = Keys::new(secret_key);

        let txid =
            Txid::from_str("3faa6bff53689b9763ed77fc693831a14030977f0ea79411b1132d27135eb1a9")
                .unwrap();
        let utxo = OutPoint::new(txid, 0);
        assert_eq!(
            LabelKind::Utxo(utxo)
                .generate_identifier(&shared_keys)
                .unwrap(),
            String::from("2666dc6af5686c709f757a6d31f0f394")
        );

        let address = Address::from_str("bc1qzqhj36c0ctkty36eqdac9q0gv9lrmnanyff0sn").unwrap();
        assert_eq!(
            LabelKind::Address(address)
                .generate_identifier(&shared_keys)
                .unwrap(),
            String::from("f225b2d56e21560d31ef180f5ff144c2")
        );
    }
}
