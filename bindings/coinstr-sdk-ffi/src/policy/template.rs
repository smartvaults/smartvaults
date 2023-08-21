// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::ops::Deref;
use std::sync::Arc;

use coinstr_sdk::core;
use coinstr_sdk::core::bitcoin::absolute::LockTime as AbsoluteLockTime;
use coinstr_sdk::core::bitcoin::Sequence;
use coinstr_sdk::core::miniscript::DescriptorPublicKey;

use crate::Descriptor;

pub struct RecoveryTemplate {
    inner: core::RecoveryTemplate,
}

impl Deref for RecoveryTemplate {
    type Target = core::RecoveryTemplate;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl RecoveryTemplate {
    pub fn social_recovery(threshold: u64, keys: Vec<Arc<Descriptor>>, after: u32) -> Self {
        let keys: Vec<DescriptorPublicKey> = keys
            .into_iter()
            .map(|k| k.as_ref().deref().clone())
            .collect();
        Self {
            inner: core::RecoveryTemplate::social_recovery(
                threshold as usize,
                keys,
                AbsoluteLockTime::from_consensus(after),
            ),
        }
    }

    pub fn inheritance(threshold: u64, keys: Vec<Arc<Descriptor>>, older: u32) -> Self {
        let keys: Vec<DescriptorPublicKey> = keys
            .into_iter()
            .map(|k| k.as_ref().deref().clone())
            .collect();
        Self {
            inner: core::RecoveryTemplate::inheritance(threshold as usize, keys, Sequence(older)),
        }
    }
}

pub struct PolicyTemplate {
    inner: core::PolicyTemplate,
}

impl Deref for PolicyTemplate {
    type Target = core::PolicyTemplate;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<core::PolicyTemplate> for PolicyTemplate {
    fn from(inner: core::PolicyTemplate) -> Self {
        Self { inner }
    }
}

impl PolicyTemplate {
    pub fn multisig(threshold: u64, keys: Vec<Arc<Descriptor>>) -> Self {
        let keys: Vec<DescriptorPublicKey> = keys
            .into_iter()
            .map(|k| k.as_ref().deref().clone())
            .collect();
        Self {
            inner: core::PolicyTemplate::multisig(threshold as usize, keys),
        }
    }

    pub fn recovery(my_key: Arc<Descriptor>, recovery: Arc<RecoveryTemplate>) -> Self {
        Self {
            inner: core::PolicyTemplate::recovery(
                my_key.as_ref().deref().clone(),
                recovery.as_ref().deref().clone(),
            ),
        }
    }

    pub fn hold(my_key: Arc<Descriptor>, after: u32) -> Self {
        Self {
            inner: core::PolicyTemplate::hold(
                my_key.as_ref().deref().clone(),
                AbsoluteLockTime::from_consensus(after),
            ),
        }
    }
}
