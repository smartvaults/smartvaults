// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::str::FromStr;

use smartvaults_sdk::core::miniscript::DescriptorPublicKey;
use uniffi::Object;

use crate::error::Result;

#[derive(Object)]
pub struct Descriptor {
    inner: DescriptorPublicKey,
}

impl Deref for Descriptor {
    type Target = DescriptorPublicKey;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<DescriptorPublicKey> for Descriptor {
    fn from(inner: DescriptorPublicKey) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl Descriptor {
    #[uniffi::constructor]
    pub fn parse(descriptor: String) -> Result<Self> {
        Ok(Self {
            inner: DescriptorPublicKey::from_str(&descriptor)?,
        })
    }

    pub fn to_str(&self) -> String {
        self.inner.to_string()
    }
}
