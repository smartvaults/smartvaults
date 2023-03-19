use std::str::FromStr;

use keechain_core::bitcoin::secp256k1::rand::rngs::OsRng;
use keechain_core::bitcoin::secp256k1::SECP256K1;
use keechain_core::bitcoin::XOnlyPublicKey;
pub use keechain_core::util::*;

const PUBLIC_KEY_LEN: usize = 66;
const HEX_CHARS: &str = "ABCDEFabcdef0123456789";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Secp256k1(#[from] nostr_sdk::secp256k1::Error),
}

pub fn extract_public_keys<S>(descriptor: S) -> Result<Vec<XOnlyPublicKey>, Error>
where
    S: Into<String>,
{
    let descriptor: String = descriptor.into();
    let len: usize = descriptor.len();
    let mut public_keys: Vec<XOnlyPublicKey> = Vec::new();
    for (index, _char) in descriptor.char_indices() {
        if len - index < PUBLIC_KEY_LEN {
            break;
        }
        if let Some(chunk) = descriptor.get(index..index + PUBLIC_KEY_LEN) {
            if maybe_pubkey(chunk) {
                if let Ok(pubkey) = XOnlyPublicKey::from_str(&chunk[2..]) {
                    public_keys.push(pubkey);
                }
            }
        }
    }
    Ok(public_keys)
}

fn maybe_pubkey(chunk: &str) -> bool {
    if chunk.len() != 66 {
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
        let descriptor = "thresh(2,pk(03e69d88524a5669723b473523cd2c6bfe76d6c289656c3ecd7981fa8fef784dcc),pk(03101e7953a54b18d0f41ea199b9adf2d7e643441b5af8e539531e6d7275cee1df),pk(027b9eda7669b1075c0eb4b117a34de19be4b3c8b0d5537b5de7fa9793b0a8e9ff))";
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
        let descriptor = "wsh(multi(2,03e69d88524a5669723b473523cd2c6bfe76d6c289656c3ecd7981fa8fef784dcc,03101e7953a54b18d0f41ea199b9adf2d7e643441b5af8e539531e6d7275cee1df,027b9eda7669b1075c0eb4b117a34de19be4b3c8b0d5537b5de7fa9793b0a8e9ff))#lrsyq0eg";
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
}
