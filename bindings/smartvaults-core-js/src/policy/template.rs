// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;

use smartvaults_core::bitcoin::absolute;
use smartvaults_core::{
    DecayingTime, Locktime, PolicyTemplate, PolicyTemplateType, RecoveryTemplate, Sequence,
};
use wasm_bindgen::prelude::*;

use crate::descriptor::JsDescriptorPublicKey;
use crate::error::{into_err, Result};

#[wasm_bindgen(js_name = RelativeLockTime)]
pub struct JsRelativeLockTime {
    inner: Sequence,
}

#[wasm_bindgen(js_class = RelativeLockTime)]
impl JsRelativeLockTime {
    #[wasm_bindgen(js_name = fromBlocks)]
    pub fn from_blocks(blocks: u16) -> Self {
        Self {
            inner: Sequence::from_height(blocks),
        }
    }
}

#[wasm_bindgen(js_name = AbsoluteLockTime)]
pub struct JsAbsoluteLockTime {
    inner: absolute::LockTime,
}

#[wasm_bindgen(js_class = AbsoluteLockTime)]
impl JsAbsoluteLockTime {
    #[wasm_bindgen(js_name = fromHeight)]
    pub fn from_height(height: u32) -> Result<JsAbsoluteLockTime> {
        Ok(Self {
            inner: absolute::LockTime::from_height(height).map_err(into_err)?,
        })
    }

    #[wasm_bindgen(js_name = fromTimestamp)]
    pub fn from_timestamp(timestamp: u32) -> Result<JsAbsoluteLockTime> {
        Ok(Self {
            inner: absolute::LockTime::from_time(timestamp).map_err(into_err)?,
        })
    }
}

#[wasm_bindgen(js_name = Locktime)]
pub struct JsLocktime {
    inner: Locktime,
}

#[wasm_bindgen(js_class = Locktime)]
impl JsLocktime {
    /// An absolute locktime restriction
    pub fn after(after: &JsAbsoluteLockTime) -> Self {
        Self {
            inner: Locktime::After(after.inner),
        }
    }

    /// A relative locktime restriction
    pub fn older(older: &JsRelativeLockTime) -> Self {
        Self {
            inner: Locktime::Older(older.inner),
        }
    }
}

#[wasm_bindgen(js_name = DecayingTime)]
pub struct JsDecayingTime {
    inner: DecayingTime,
}

#[wasm_bindgen(js_class = DecayingTime)]
impl JsDecayingTime {
    pub fn single(timelock: &JsLocktime) -> Self {
        Self {
            inner: DecayingTime::Single(timelock.inner),
        }
    }

    pub fn multiple(timelocks: Vec<JsLocktime>) -> Self {
        Self {
            inner: DecayingTime::Multiple(timelocks.into_iter().map(|l| l.inner).collect()),
        }
    }
}

#[wasm_bindgen(js_name = PolicyTemplateType)]
pub enum JsPolicyTemplateType {
    Singlesig,
    Multisig,
    /// Social Recovery / Inheritance
    Recovery,
    Hold,
    Decaying,
}

impl From<PolicyTemplateType> for JsPolicyTemplateType {
    fn from(value: PolicyTemplateType) -> Self {
        match value {
            PolicyTemplateType::Multisig => Self::Multisig,
            PolicyTemplateType::Recovery => Self::Recovery,
            PolicyTemplateType::Hold => Self::Hold,
            PolicyTemplateType::Decaying => Self::Decaying,
        }
    }
}

#[wasm_bindgen(js_name = RecoveryTemplate)]
pub struct JsRecoveryTemplate {
    inner: RecoveryTemplate,
}

#[wasm_bindgen(js_class = RecoveryTemplate)]
impl JsRecoveryTemplate {
    #[wasm_bindgen(constructor)]
    pub fn new(threshold: usize, keys: Vec<JsDescriptorPublicKey>, timelock: &JsLocktime) -> Self {
        Self {
            inner: RecoveryTemplate::new(
                threshold,
                keys.into_iter().map(|d| d.into()),
                timelock.inner,
            ),
        }
    }
}

/// Policy template
#[wasm_bindgen(js_name = PolicyTemplate)]
pub struct JsPolicyTemplate {
    inner: PolicyTemplate,
}

impl Deref for JsPolicyTemplate {
    type Target = PolicyTemplate;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

#[wasm_bindgen(js_class = PolicyTemplate)]
impl JsPolicyTemplate {
    pub fn singlesig(key: &JsDescriptorPublicKey) -> Self {
        Self {
            inner: PolicyTemplate::singlesig(key.deref().clone()),
        }
    }

    pub fn multisig(threshold: usize, keys: Vec<JsDescriptorPublicKey>) -> Self {
        Self {
            inner: PolicyTemplate::multisig(
                threshold,
                keys.into_iter().map(|d| d.into()).collect(),
            ),
        }
    }

    pub fn recovery(my_key: &JsDescriptorPublicKey, recovery: &JsRecoveryTemplate) -> Self {
        Self {
            inner: PolicyTemplate::recovery(my_key.deref().clone(), recovery.inner.clone()),
        }
    }

    pub fn hold(my_key: &JsDescriptorPublicKey, timelock: &JsLocktime) -> Self {
        Self {
            inner: PolicyTemplate::hold(my_key.deref().clone(), timelock.inner),
        }
    }

    pub fn decaying(
        start_threshold: usize,
        keys: Vec<JsDescriptorPublicKey>,
        time: &JsDecayingTime,
    ) -> Self {
        Self {
            inner: PolicyTemplate::decaying(
                start_threshold,
                keys.into_iter().map(|d| d.into()).collect(),
                time.inner.clone(),
            ),
        }
    }
}
