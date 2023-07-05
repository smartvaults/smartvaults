// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::signer::{self, SignerType};

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

    pub fn descriptor(&self) -> String {
        self.inner.descriptor().to_string()
    }

    pub fn signer_type(&self) -> SignerType {
        self.inner.signer_type()
    }

    #[allow(clippy::inherent_to_string)]
    pub fn to_string(&self) -> String {
        self.inner.to_string()
    }
}
