// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Versioned de/serialization schema

use thiserror::Error;

/// Error
#[derive(Debug, Error)]
pub enum Error {
    /// Unknown version
    #[error("unknown version: {0}")]
    UnknownVersion(u8),
    /// Wrong schema
    #[error("wrong schema")]
    WrongSchema,
}

pub type SchemaVersion = Version;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum Version {
    /// Protocol Buffers
    #[default]
    ProtoBuf = 0x01,
}

impl Version {
    /// Get [`Version`] as `u8`
    pub fn as_u8(&self) -> u8 {
        *self as u8
    }
}

impl TryFrom<u8> for Version {
    type Error = Error;

    fn try_from(version: u8) -> Result<Self, Self::Error> {
        match version {
            0x01 => Ok(Self::ProtoBuf),
            v => Err(Error::UnknownVersion(v)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Schema<'a> {
    pub version: Version,
    pub data: &'a [u8],
}

pub fn encode<T>(data: T, version: Version) -> Vec<u8>
where
    T: AsRef<[u8]>,
{
    let data: &[u8] = data.as_ref();
    let mut payload: Vec<u8> = Vec::with_capacity(1 + data.len());
    payload.push(version.as_u8());
    payload.extend_from_slice(data);
    payload
}

pub fn decode(payload: &[u8]) -> Result<Schema<'_>, Error> {
    if let Some((&version, data)) = payload.split_first() {
        Ok(Schema {
            version: Version::try_from(version)?,
            data,
        })
    } else {
        Err(Error::WrongSchema)
    }
}
