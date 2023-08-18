// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::core::signer::{self, SignerType};
use coinstr_sdk::db::model;
use nostr_ffi::{EventId, PublicKey};

use crate::error::Result;

pub struct GetSigner {
    inner: model::GetSigner,
}

impl From<model::GetSigner> for GetSigner {
    fn from(inner: model::GetSigner) -> Self {
        Self { inner }
    }
}

impl GetSigner {
    pub fn signer_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.signer_id.into())
    }

    pub fn signer(&self) -> Arc<Signer> {
        Arc::new(self.inner.signer.clone().into())
    }
}

pub struct Signer {
    inner: signer::Signer,
}

impl From<signer::Signer> for Signer {
    fn from(inner: signer::Signer) -> Self {
        Self { inner }
    }
}

impl Signer {
    pub fn name(&self) -> String {
        self.inner.name()
    }

    pub fn fingerprint(&self) -> String {
        self.inner.fingerprint().to_string()
    }

    pub fn descriptor(&self) -> Result<String> {
        Ok(self.inner.descriptor_public_key()?.to_string())
    }

    pub fn signer_type(&self) -> SignerType {
        self.inner.signer_type()
    }

    pub fn display(&self) -> String {
        self.inner.to_string()
    }
}

pub struct GetSharedSigner {
    inner: model::GetSharedSigner,
}

impl From<model::GetSharedSigner> for GetSharedSigner {
    fn from(inner: model::GetSharedSigner) -> Self {
        Self { inner }
    }
}

impl GetSharedSigner {
    pub fn shared_signer_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.shared_signer_id.into())
    }

    pub fn owner_public_key(&self) -> Arc<PublicKey> {
        Arc::new(self.inner.owner_public_key.into())
    }

    pub fn shared_signer(&self) -> Arc<SharedSigner> {
        Arc::new(self.inner.shared_signer.clone().into())
    }
}

pub struct SharedSigner {
    inner: signer::SharedSigner,
}

impl From<signer::SharedSigner> for SharedSigner {
    fn from(inner: signer::SharedSigner) -> Self {
        Self { inner }
    }
}

impl SharedSigner {
    pub fn fingerprint(&self) -> String {
        self.inner.fingerprint().to_string()
    }

    pub fn descriptor(&self) -> Result<String> {
        Ok(self.inner.descriptor_public_key()?.to_string())
    }
}
