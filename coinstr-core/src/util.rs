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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_extract_public_keys() {
        let descriptor = "thresh(2,pk(02e69d88524a5669723b473523cd2c6bfe76d6c289656c3ecd7981fa8fef784dcc),pk(02101e7953a54b18d0f41ea199b9adf2d7e643441b5af8e539531e6d7275cee1df),pk(02ea527e059759d368a55253270454e58e9d6e4fe2e98d302d6e01821fa973259d))";
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
                    "ea527e059759d368a55253270454e58e9d6e4fe2e98d302d6e01821fa973259d"
                )
                .unwrap(),
            ]
        )
    }
}
