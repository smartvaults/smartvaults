// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::sync::Arc;

use nostr_ffi::EventId;
use smartvaults_sdk::core::signer;
use smartvaults_sdk::types;
use uniffi::{Enum, Object};

use crate::error::Result;
use crate::{Descriptor, User};

#[derive(Enum)]
pub enum SignerType {
    Seed,
    Hardware,
    AirGap,
}

impl From<signer::SignerType> for SignerType {
    fn from(value: signer::SignerType) -> Self {
        match value {
            signer::SignerType::Seed => Self::Seed,
            signer::SignerType::Hardware => Self::Hardware,
            signer::SignerType::AirGap => Self::AirGap,
        }
    }
}

#[derive(Object)]
pub struct GetSigner {
    inner: types::GetSigner,
}

impl From<types::GetSigner> for GetSigner {
    fn from(inner: types::GetSigner) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl GetSigner {
    pub fn signer_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.signer_id.into())
    }

    pub fn signer(&self) -> Arc<Signer> {
        Arc::new(self.inner.signer.clone().into())
    }
}

#[derive(Object)]
pub struct Signer {
    inner: signer::Signer,
}

impl Deref for Signer {
    type Target = signer::Signer;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<signer::Signer> for Signer {
    fn from(inner: signer::Signer) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl Signer {
    pub fn name(&self) -> String {
        self.inner.name()
    }

    pub fn fingerprint(&self) -> String {
        self.inner.fingerprint().to_string()
    }

    pub fn descriptor(&self) -> Result<Arc<Descriptor>> {
        Ok(Arc::new(self.inner.descriptor_public_key()?.into()))
    }

    pub fn signer_type(&self) -> SignerType {
        self.inner.signer_type().into()
    }

    pub fn display(&self) -> String {
        self.inner.to_string()
    }
}

#[derive(Object)]
pub struct GetSharedSigner {
    inner: types::GetSharedSigner,
}

impl From<types::GetSharedSigner> for GetSharedSigner {
    fn from(inner: types::GetSharedSigner) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl GetSharedSigner {
    pub fn shared_signer_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.shared_signer_id.into())
    }

    pub fn owner(&self) -> Arc<User> {
        Arc::new(self.inner.owner.clone().into())
    }

    pub fn shared_signer(&self) -> Arc<SharedSigner> {
        Arc::new(self.inner.shared_signer.clone().into())
    }
}

#[derive(Object)]
pub struct SharedSigner {
    inner: signer::SharedSigner,
}

impl From<signer::SharedSigner> for SharedSigner {
    fn from(inner: signer::SharedSigner) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl SharedSigner {
    pub fn fingerprint(&self) -> String {
        self.inner.fingerprint().to_string()
    }

    pub fn descriptor(&self) -> Result<Arc<Descriptor>> {
        Ok(Arc::new(self.inner.descriptor_public_key()?.into()))
    }
}
