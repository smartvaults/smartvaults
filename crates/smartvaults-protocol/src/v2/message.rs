// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Protocol Message
//!
//! ```notrust
//!  Protocol version    Encoding version (Protobuf, JSON, ..)        Encoded object
//!   |                              |                                      |
//! |---|                         |----|       |-------------------------------------------------------|
//! [0x01, 0xF9, 0xBE, 0xB4, 0xD9, 0x01, 0x01, 0x00, 0xB4, 0xAA, 0x19 0xF4, 0x39, 0x00, 0x12, 0x21, ...]
//!       |----------------------|      |----|
//!                 |                      |
//!           Network magic          Object version (?)
//! ```

use nostr::nips::nip44;
use nostr::Keys;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::secp256k1::{SecretKey, XOnlyPublicKey};
use thiserror::Error;

use crate::v2::{self, NetworkMagic};

/// Protocol Message Error
#[derive(Debug, Error)]
pub enum ProtocolMessageError {
    #[error(transparent)]
    Network(#[from] v2::network::Error),
    /// Unknown protocol version
    #[error("unknown protocol version: {0}")]
    UnknownProtocolVersion(u8),
    /// Unknown encoding version
    #[error("unknown encoding version: {0}")]
    UnknownEncodingVersion(u8),
    /// Invalid protocol schema
    #[error("invalid protocol message")]
    InvalidProtocolMessage,
}

/// Smart Vaults Protocol Version
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ProtocolVersion {
    /// Genesis - Version 1
    #[default]
    Genesis = 0x01,
}

impl ProtocolVersion {
    /// Get [ProtocolVersion] as `u8`
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl TryFrom<u8> for ProtocolVersion {
    type Error = ProtocolMessageError;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            0x01 => Ok(Self::Genesis),
            v => Err(ProtocolMessageError::UnknownProtocolVersion(v)),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum EncodingVersion {
    /// Protocol Buffers
    #[default]
    ProtoBuf = 0x01,
}

impl EncodingVersion {
    /// Get [EncodingVersion] as `u8`
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl TryFrom<u8> for EncodingVersion {
    type Error = ProtocolMessageError;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            0x01 => Ok(Self::ProtoBuf),
            v => Err(ProtocolMessageError::UnknownEncodingVersion(v)),
        }
    }
}

/// Smart Vaults Protocol Message
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProtocolMessage<'a> {
    pub version: ProtocolVersion,
    pub network: Network,
    pub encoding_version: EncodingVersion,
    pub data: &'a [u8],
}

impl<'a> ProtocolMessage<'a> {
    pub fn encode(&self) -> Vec<u8> {
        // Protocol version (1) + Network magic (4) + Encoding version (1) + data
        let mut payload: Vec<u8> = Vec::with_capacity(1 + 4 + 1 + self.data.len());
        payload.push(self.version.as_u8());
        payload.extend_from_slice(&self.network.magic().to_bytes());
        payload.push(self.encoding_version.as_u8());
        payload.extend_from_slice(self.data);
        payload
    }

    pub fn decode(payload: &'a [u8]) -> Result<Self, ProtocolMessageError> {
        // Check if payload is >= 7 (Protocol + network magic + encoding version + at least 2 byte of data)
        if payload.len() >= 8 {
            let version: ProtocolVersion = ProtocolVersion::try_from(payload[0])?;
            let network: NetworkMagic = NetworkMagic::from_slice(&payload[1..5])?;
            let encoding_version: EncodingVersion = EncodingVersion::try_from(payload[5])?;
            Ok(ProtocolMessage {
                version,
                network: *network,
                encoding_version,
                data: &payload[6..],
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

    /// Define protocol version
    fn protocol_version(&self) -> ProtocolVersion {
        ProtocolVersion::default()
    }

    /// Define network magic
    fn protocol_network(&self) -> Network;

    /// Encode protocol message
    fn encode(&self) -> Vec<u8> {
        let (encoding_version, data): (EncodingVersion, Vec<u8>) = self.pre_encoding();
        let message: ProtocolMessage = ProtocolMessage {
            version: self.protocol_version(),
            network: self.protocol_network(),
            encoding_version,
            data: &data,
        };
        message.encode()
    }

    /// Pre-encoding of protocol message
    fn pre_encoding(&self) -> (EncodingVersion, Vec<u8>);

    /// Decode `payload`
    fn decode<T>(payload: T) -> Result<Self, Self::Err>
    where
        T: AsRef<[u8]>,
        <Self as ProtocolEncoding>::Err: From<ProtocolMessageError>,
    {
        let ProtocolMessage {
            encoding_version,
            data,
            ..
        } = ProtocolMessage::decode(payload.as_ref())?;
        // TODO: check protocol version
        // TODO: check network magic before full deserialization
        match encoding_version {
            EncodingVersion::ProtoBuf => Self::decode_protobuf(data),
        }
    }

    /// Decode protobuf data
    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err>;
}

/// Protocol encryption
pub trait ProtocolEncryption: ProtocolEncoding
where
    <Self as ProtocolEncoding>::Err: From<ProtocolMessageError>,
    <Self as ProtocolEncryption>::Err:
        From<<Self as ProtocolEncoding>::Err> + From<nip44::Error> + From<nostr::key::Error>,
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
        let payload: String = nip44::decrypt(secret_key, public_key, payload)?;
        Ok(Self::decode(payload)?)
    }

    /// Encrypt
    fn encrypt(
        &self,
        secret_key: &SecretKey,
        public_key: &XOnlyPublicKey,
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
        Self::decrypt(&keys.secret_key()?, &keys.public_key(), payload)
    }

    /// Encrypt with [`Keys`] (for self-encryption)
    fn encrypt_with_keys(&self, keys: &Keys) -> Result<String, <Self as ProtocolEncryption>::Err> {
        self.encrypt(&keys.secret_key()?, &keys.public_key())
    }
}
