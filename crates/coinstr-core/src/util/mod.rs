// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use keechain_core::bitcoin::secp256k1::rand::rngs::OsRng;
use keechain_core::bitcoin::XOnlyPublicKey;
pub use keechain_core::util::*;
use keechain_core::SECP256K1;

pub mod encryption;
pub mod serde;

pub use self::encryption::{Encryption, EncryptionError};
pub use self::serde::Serde;

pub trait Unspendable {
    fn unspendable() -> Self;
}

impl Unspendable for XOnlyPublicKey {
    fn unspendable() -> Self {
        let mut rng = OsRng::default();
        let (_, public_key) = SECP256K1.generate_keypair(&mut rng);
        let (public_key, _) = public_key.x_only_public_key();
        public_key
    }
}
