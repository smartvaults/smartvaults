// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_core::secp256k1::{SecretKey, XOnlyPublicKey};

pub mod constants;
pub mod crypto;
pub mod identifier;
mod network;
pub mod schema;
pub mod shared_key;
pub mod vault;

pub use self::identifier::Identifier;
pub use self::network::NetworkMagic;
pub use self::shared_key::SharedKey;
pub use self::vault::Vault;

pub trait ProtocolEncoding: Sized {
    type Err;

    fn decode<T>(payload: T) -> Result<Self, Self::Err>
    where
        T: AsRef<[u8]>;

    fn encode(&self) -> Vec<u8>;
}

pub trait ProtocolEncryption: ProtocolEncoding
where
    <Self as ProtocolEncryption>::Err: From<<Self as ProtocolEncoding>::Err>,
    <Self as ProtocolEncryption>::Err: From<crypto::Error>,
{
    type Err;

    fn decrypt<T>(
        secret_key: &SecretKey,
        public_key: &XOnlyPublicKey,
        payload: T,
    ) -> Result<Self, <Self as ProtocolEncryption>::Err>
    where
        T: AsRef<[u8]>,
    {
        let payload: Vec<u8> = crypto::decrypt(secret_key, public_key, payload)?;
        Ok(Self::decode(payload)?)
    }

    fn encrypt(
        &self,
        secret_key: &SecretKey,
        public_key: &XOnlyPublicKey,
    ) -> Result<String, <Self as ProtocolEncryption>::Err> {
        let buf: Vec<u8> = self.encode();
        Ok(crypto::encrypt(
            secret_key,
            public_key,
            buf,
            crypto::Version::XChaCha20Poly1305,
        )?)
    }
}
