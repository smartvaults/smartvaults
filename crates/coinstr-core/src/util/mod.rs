// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use keechain_core::bitcoin::secp256k1::rand::rngs::OsRng;
use keechain_core::secp256k1::{Secp256k1, Signing, XOnlyPublicKey};
pub use keechain_core::util::*;

pub mod encryption;
pub mod serde;

pub use self::encryption::{Encryption, EncryptionError};
pub use self::serde::Serde;

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
