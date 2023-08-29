// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use ::serde::{Deserialize, Deserializer, Serializer};
use bdk::bitcoin::psbt::PartiallySignedTransaction;
use keechain_core::bitcoin::secp256k1::rand::rngs::OsRng;
use keechain_core::secp256k1::{Secp256k1, Signing, XOnlyPublicKey};
pub use keechain_core::util::*;

pub trait Unspendable {
    fn unspendable<C>(secp: &Secp256k1<C>) -> Self
    where
        C: Signing;
}

impl Unspendable for XOnlyPublicKey {
    fn unspendable<C>(secp: &Secp256k1<C>) -> Self
    where
        C: Signing,
    {
        let mut rng = OsRng;
        let (_, public_key) = secp.generate_keypair(&mut rng);
        let (public_key, _) = public_key.x_only_public_key();
        public_key
    }
}

pub(crate) fn serialize_psbt<S>(
    psbt: &PartiallySignedTransaction,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&psbt.to_string())
}

pub(crate) fn deserialize_psbt<'de, D>(
    deserializer: D,
) -> Result<PartiallySignedTransaction, D::Error>
where
    D: Deserializer<'de>,
{
    let psbt = String::deserialize(deserializer)?;
    PartiallySignedTransaction::from_str(&psbt).map_err(::serde::de::Error::custom)
}
