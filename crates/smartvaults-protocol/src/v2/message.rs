// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Protocol Message
//!
//! ```notrust
//! Message version (Protobuf, JSON, ...)        Encoded object
//!    |                                                 |
//! |----||--------------------------------------------------------------|
//! [0x01, 0x01, 0x00, 0xB4, 0xAA, 0x19 0xF4, 0x39, 0x00, 0x12, 0x21, ...]
//! ```

use async_trait::async_trait;
use nostr::nips::nip44;
use nostr::{Keys, PublicKey, SecretKey};
use nostr_signer::NostrSigner;
use thiserror::Error;

/// Protocol Message Error
#[derive(Debug, Error)]
pub enum ProtocolMessageError {
    /// Unknown message version
    #[error("unknown message version: {0}")]
    UnknownMessageVersion(u8),
    /// Invalid protocol schema
    #[error("invalid protocol message")]
    InvalidProtocolMessage,
}

/// Protocol Encoding Version
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum MessageVersion {
    /// Protocol Buffers
    #[default]
    ProtoBuf = 0x01,
}

impl MessageVersion {
    /// Get [MessageVersion] as `u8`
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl TryFrom<u8> for MessageVersion {
    type Error = ProtocolMessageError;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            0x01 => Ok(Self::ProtoBuf),
            v => Err(ProtocolMessageError::UnknownMessageVersion(v)),
        }
    }
}

/// Smart Vaults Protocol Message
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ProtocolMessage<'a> {
    version: MessageVersion,
    data: &'a [u8],
}

impl<'a> ProtocolMessage<'a> {
    fn encode(&self) -> Vec<u8> {
        let mut payload: Vec<u8> = Vec::with_capacity(1 + self.data.len());
        payload.push(self.version.as_u8());
        payload.extend_from_slice(self.data);
        payload
    }

    fn decode(payload: &'a [u8]) -> Result<Self, ProtocolMessageError> {
        // Check if payload is >= 3 (encoding version + at least 2 byte of data)
        if payload.len() >= 3 {
            let version: MessageVersion = MessageVersion::try_from(payload[0])?;
            Ok(ProtocolMessage {
                version,
                data: &payload[1..],
            })
        } else {
            Err(ProtocolMessageError::InvalidProtocolMessage)
        }
    }
}

/// Protocol encoding/decoding
pub trait ProtocolEncoding: Sized {
    /// Error
    type Err;

    /// Encode protocol message
    fn encode(&self) -> Vec<u8> {
        let (version, data): (MessageVersion, Vec<u8>) = self.pre_encoding();
        let message: ProtocolMessage = ProtocolMessage {
            version,
            data: &data,
        };
        message.encode()
    }

    /// Pre-encoding of protocol message
    fn pre_encoding(&self) -> (MessageVersion, Vec<u8>);

    /// Decode `payload`
    fn decode<T>(payload: T) -> Result<Self, Self::Err>
    where
        T: AsRef<[u8]>,
        <Self as ProtocolEncoding>::Err: From<ProtocolMessageError>,
    {
        let ProtocolMessage { version, data } = ProtocolMessage::decode(payload.as_ref())?;

        match version {
            MessageVersion::ProtoBuf => Self::decode_protobuf(data),
        }
    }

    /// Decode protobuf data
    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err>;
}

/// Protocol encryption
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait ProtocolEncryption: ProtocolEncoding
where
    <Self as ProtocolEncoding>::Err: From<ProtocolMessageError>,
    <Self as ProtocolEncryption>::Err:
        From<<Self as ProtocolEncoding>::Err> + From<nip44::Error> + From<nostr::key::Error>,
    <Self as ProtocolEncryption>::Err: From<nostr_signer::Error>,
{
    /// Error
    type Err;

    /// Decrypt
    fn decrypt<T>(
        secret_key: &SecretKey,
        public_key: &PublicKey,
        payload: T,
    ) -> Result<Self, <Self as ProtocolEncryption>::Err>
    where
        T: AsRef<[u8]>,
    {
        let payload: Vec<u8> = nip44::decrypt_to_bytes(secret_key, public_key, payload)?;
        Ok(Self::decode(payload)?)
    }

    /// Encrypt
    fn encrypt(
        &self,
        secret_key: &SecretKey,
        public_key: &PublicKey,
    ) -> Result<String, <Self as ProtocolEncryption>::Err> {
        let buf: Vec<u8> = self.encode();
        Ok(nip44::encrypt(
            secret_key,
            public_key,
            buf,
            nip44::Version::V2,
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
        Self::decrypt(keys.secret_key()?, &keys.public_key(), payload)
    }

    /// Encrypt with [`Keys`] (for self-encryption)
    fn encrypt_with_keys(&self, keys: &Keys) -> Result<String, <Self as ProtocolEncryption>::Err> {
        self.encrypt(keys.secret_key()?, &keys.public_key())
    }

    /// Decrypt with [`NostrSigner`]
    async fn decrypt_with_signer_and_public_key(
        signer: &NostrSigner,
        public_key: &PublicKey,
        payload: &[u8],
    ) -> Result<Self, <Self as ProtocolEncryption>::Err> {
        let payload: String = signer.nip44_decrypt(*public_key, payload).await?;
        let payload: &[u8] = payload.as_bytes();
        Ok(Self::decode(payload)?)
    }

    /// Encrypt with [`NostrSigner`]
    async fn encrypt_with_signer_and_public_key(
        &self,
        signer: &NostrSigner,
        public_key: &PublicKey,
    ) -> Result<String, <Self as ProtocolEncryption>::Err> {
        let buf: Vec<u8> = self.encode();
        Ok(signer.nip44_encrypt(*public_key, buf).await?)
    }

    /// Decrypt with [`NostrSigner`] (for self-decryption)
    async fn decrypt_with_signer(
        signer: &NostrSigner,
        payload: &[u8],
    ) -> Result<Self, <Self as ProtocolEncryption>::Err> {
        let public_key = signer.public_key().await?;
        Self::decrypt_with_signer_and_public_key(signer, &public_key, payload).await
    }

    /// Encrypt with [`NostrSigner`] (for self-encryption)
    async fn encrypt_with_signer(
        &self,
        signer: &NostrSigner,
    ) -> Result<String, <Self as ProtocolEncryption>::Err> {
        let public_key = signer.public_key().await?;
        self.encrypt_with_signer_and_public_key(signer, &public_key)
            .await
    }
}
