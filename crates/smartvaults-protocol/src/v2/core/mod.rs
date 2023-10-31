// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr::Keys;
use smartvaults_core::secp256k1::{SecretKey, XOnlyPublicKey};

mod crypto;
mod schema;

pub use self::crypto::Error as CryptoError;
use self::schema::Schema;
pub use self::schema::{Error as SchemaError, SchemaVersion};

/// Protocol encoding/decoding
pub trait ProtocolEncoding: Sized {
    /// Error
    type Err;

    /// Encode
    fn encode(&self) -> Vec<u8> {
        let (version, buf): (SchemaVersion, Vec<u8>) = self.pre_encoding();
        schema::encode(buf, version)
    }

    /// Pre-encoding (not include the schema version)
    fn pre_encoding(&self) -> (SchemaVersion, Vec<u8>);

    /// Decode `payload`
    fn decode<T>(payload: T) -> Result<Self, Self::Err>
    where
        T: AsRef<[u8]>,
        <Self as ProtocolEncoding>::Err: From<schema::Error>,
    {
        let Schema { version, data } = schema::decode(payload.as_ref())?;
        match version {
            SchemaVersion::ProtoBuf => Self::decode_protobuf(data),
        }
    }

    /// Decode protobuf data
    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err>;
}

/// Protocol encryption
pub trait ProtocolEncryption: ProtocolEncoding
where
    <Self as ProtocolEncoding>::Err: From<schema::Error>,
    <Self as ProtocolEncryption>::Err:
        From<<Self as ProtocolEncoding>::Err> + From<CryptoError> + From<nostr::key::Error>,
{
    /// Error
    type Err;

    /// Decrypt
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

    /// Encrypt
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
            crypto::Version::default(),
        )?)
    }

    /// Decrypt with [`Keys`] (for self-decryption)
    fn decrypt_with_keys<T>(
        keys: &Keys,
        payload: T,
    ) -> Result<Self, <Self as ProtocolEncryption>::Err>
    where
        T: AsRef<[u8]>,
    {
        Self::decrypt(&keys.secret_key()?, &keys.public_key(), payload)
    }

    /// Encrypt with [`Keys`] (for self-encryption)
    fn encrypt_with_keys(&self, keys: &Keys) -> Result<String, <Self as ProtocolEncryption>::Err> {
        self.encrypt(&keys.secret_key()?, &keys.public_key())
    }
}
