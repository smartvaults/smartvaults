use std::str::FromStr;

use bdk::bitcoin::XOnlyPublicKey;
pub use keechain_core::util::*;

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
    let splitted = descriptor.split("pk(");
    let mut public_keys: Vec<XOnlyPublicKey> = Vec::new();
    for chunk in splitted.into_iter() {
        if let Some(pubkey) = chunk.get(2..66) {
            let pubkey = XOnlyPublicKey::from_str(pubkey)?;
            public_keys.push(pubkey);
        }
    }
    Ok(public_keys)
}
