//! Example of extracting public keys from a descriptor

use coinstr_core::util;

fn main() {
    let descriptor = "thresh(2,pk(02e69d88524a5669723b473523cd2c6bfe76d6c289656c3ecd7981fa8fef784dcc),pk(02101e7953a54b18d0f41ea199b9adf2d7e643441b5af8e539531e6d7275cee1df),pk(02ea527e059759d368a55253270454e58e9d6e4fe2e98d302d6e01821fa973259d))";
    let pubkeys = util::extract_public_keys(descriptor).unwrap();
    for pubkey in pubkeys.into_iter() {
        println!("{pubkey}");
    }
}
