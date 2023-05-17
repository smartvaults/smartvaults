// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;

use keechain_core::bitcoin::secp256k1::rand::rngs::OsRng;
use keechain_core::bitcoin::XOnlyPublicKey;
pub use keechain_core::util::*;
use keechain_core::SECP256K1;

pub mod encryption;
pub mod serde;

pub use self::encryption::{Encryption, EncryptionError};
pub use self::serde::Serde;

const XONLY_PUBLIC_KEY_LEN: usize = 64;
const HEX_CHARS: &str = "ABCDEFabcdef0123456789";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Secp256k1(#[from] keechain_core::bitcoin::secp256k1::Error),
}

pub fn extract_public_keys<S>(descriptor: S) -> Result<Vec<XOnlyPublicKey>, Error>
where
    S: Into<String>,
{
    let descriptor: String = descriptor.into();
    let len: usize = descriptor.len();
    let mut public_keys: Vec<XOnlyPublicKey> = Vec::new();
    for (index, _char) in descriptor.char_indices() {
        if len - index < XONLY_PUBLIC_KEY_LEN {
            break;
        }
        if let Some(chunk) = descriptor.get(index..index + XONLY_PUBLIC_KEY_LEN) {
            if maybe_xonly_pubkey(chunk) {
                if let Ok(pubkey) = XOnlyPublicKey::from_str(chunk) {
                    if !public_keys.contains(&pubkey) {
                        public_keys.push(pubkey);
                    }
                }
            }
        }
    }
    Ok(public_keys)
}

fn maybe_xonly_pubkey(chunk: &str) -> bool {
    if chunk.len() != XONLY_PUBLIC_KEY_LEN {
        return false;
    }

    for c in chunk.chars() {
        if !HEX_CHARS.contains(c) {
            return false;
        }
    }

    true
}

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_policy_extractor() {
        let descriptor = "thresh(2,pk(e69d88524a5669723b473523cd2c6bfe76d6c289656c3ecd7981fa8fef784dcc),pk(101e7953a54b18d0f41ea199b9adf2d7e643441b5af8e539531e6d7275cee1df),pk(7b9eda7669b1075c0eb4b117a34de19be4b3c8b0d5537b5de7fa9793b0a8e9ff))";
        let pubkeys = extract_public_keys(descriptor).unwrap();

        assert_eq!(
            pubkeys,
            vec![
                XOnlyPublicKey::from_str(
                    "e69d88524a5669723b473523cd2c6bfe76d6c289656c3ecd7981fa8fef784dcc"
                )
                .unwrap(),
                XOnlyPublicKey::from_str(
                    "101e7953a54b18d0f41ea199b9adf2d7e643441b5af8e539531e6d7275cee1df"
                )
                .unwrap(),
                XOnlyPublicKey::from_str(
                    "7b9eda7669b1075c0eb4b117a34de19be4b3c8b0d5537b5de7fa9793b0a8e9ff"
                )
                .unwrap(),
            ]
        )
    }

    #[test]
    fn test_descriptor_extractor() {
        let descriptor = "tr(0298e9fdeb06b3e9e49db3dbffe1a3a353bf359c54fe415769dd3f174f4ea610,multi_a(2,c04e8da91853b7fd215102e6aa48477d8e1ba6b3c16902371a153d3784a1b0f7,e8978cf935f7f912e77c57fcf03668a20cf4eacfbcdeb046613946266d8b8204))#37490v6l";
        let pubkeys = extract_public_keys(descriptor).unwrap();

        assert_eq!(
            pubkeys,
            vec![
                XOnlyPublicKey::from_str(
                    "0298e9fdeb06b3e9e49db3dbffe1a3a353bf359c54fe415769dd3f174f4ea610"
                )
                .unwrap(),
                XOnlyPublicKey::from_str(
                    "c04e8da91853b7fd215102e6aa48477d8e1ba6b3c16902371a153d3784a1b0f7"
                )
                .unwrap(),
                XOnlyPublicKey::from_str(
                    "e8978cf935f7f912e77c57fcf03668a20cf4eacfbcdeb046613946266d8b8204"
                )
                .unwrap(),
            ]
        )
    }
}
