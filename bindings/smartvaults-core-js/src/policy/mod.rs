// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;
use core::str::FromStr;

use smartvaults_core::bitcoin::bip32::Fingerprint;
use smartvaults_core::policy::Policy;
use smartvaults_core::SelectableCondition;
use wasm_bindgen::prelude::*;

pub mod template;

use self::template::JsPolicyTemplate;
use crate::error::{into_err, Result};
use crate::network::JsNetwork;

#[wasm_bindgen(js_name = SelectableCondition)]
pub struct JsSelectableCondition {
    inner: SelectableCondition,
}

impl From<SelectableCondition> for JsSelectableCondition {
    fn from(inner: SelectableCondition) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = SelectableCondition)]
impl JsSelectableCondition {
    #[wasm_bindgen(getter)]
    pub fn path(&self) -> String {
        self.inner.path.clone()
    }

    #[wasm_bindgen(getter)]
    pub fn thresh(&self) -> usize {
        self.inner.thresh
    }

    #[wasm_bindgen(getter, js_name = subPaths)]
    pub fn sub_paths(&self) -> Vec<String> {
        self.inner.sub_paths.clone()
    }
}

#[wasm_bindgen(js_name = Policy)]
pub struct JsPolicy {
    inner: Policy,
}

impl From<Policy> for JsPolicy {
    fn from(inner: Policy) -> Self {
        Self { inner }
    }
}

impl Deref for JsPolicy {
    type Target = Policy;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<JsPolicy> for Policy {
    fn from(policy: JsPolicy) -> Self {
        policy.inner
    }
}

#[wasm_bindgen(js_class = Policy)]
impl JsPolicy {
    /// Construct `Policy` from descriptor string
    ///
    /// The descriptor must be typed, for example: `tr(...)`
    #[wasm_bindgen(js_name = fromDescriptor)]
    pub fn from_descriptor(descriptor: &str, network: JsNetwork) -> Result<JsPolicy> {
        Ok(Self {
            inner: Policy::from_descriptor(descriptor, network.into()).map_err(into_err)?,
        })
    }

    /// Construct `Policy` from miniscript
    ///
    /// <https://bitcoin.sipa.be/miniscript/>
    #[wasm_bindgen(js_name = fromMiniscript)]
    pub fn from_miniscript(policy: &str, network: JsNetwork) -> Result<JsPolicy> {
        Ok(Self {
            inner: Policy::from_miniscript(policy, network.into()).map_err(into_err)?,
        })
    }

    /// Try to construct `Policy` from descriptor string or miniscript policy
    ///
    /// Internally try before to construct the `Policy` from a descriptor string. If fail, try from miniscript policy.
    #[wasm_bindgen(js_name = fromDescOrMiniscript)]
    pub fn from_desc_or_miniscript(
        desc_or_miniscript: &str,
        network: JsNetwork,
    ) -> Result<JsPolicy> {
        Ok(Self {
            inner: Policy::from_desc_or_miniscript(desc_or_miniscript, network.into())
                .map_err(into_err)?,
        })
    }

    /// Construct `Policy` from `PolicyTemplate`
    #[wasm_bindgen(js_name = fromTemplate)]
    pub fn from_template(
        template: &JsPolicyTemplate,
        network: JsNetwork,
    ) -> Result<JsPolicy> {
        Ok(Self {
            inner: Policy::from_template(
                template.deref().clone(),
                network.into(),
            )
            .map_err(into_err)?,
        })
    }

    pub fn descriptor(&self) -> String {
        self.inner.descriptor().to_string()
    }

    /// Get network
    pub fn network(&self) -> JsNetwork {
        self.inner.network().into()
    }

    /// Check if `Policy` has an `absolute` or `relative` timelock
    #[wasm_bindgen(js_name = hasTimelock)]
    pub fn has_timelock(&self) -> bool {
        self.inner.has_timelock()
    }

    /// Check if `Policy` has a `absolute` timelock
    #[wasm_bindgen(js_name = hasAbsoluteTimelock)]
    pub fn has_absolute_timelock(&self) -> bool {
        self.inner.has_absolute_timelock()
    }

    /// Check if `Policy` has a `relative` timelock
    #[wasm_bindgen(js_name = hasRelativeTimelock)]
    pub fn has_relative_timelock(&self) -> bool {
        self.inner.has_relative_timelock()
    }

    /// Get `SatisfiableItem`
    #[wasm_bindgen(js_name = satisfiableItem)]
    pub fn satisfiable_item(&self) -> Result<String> {
        let item = self.inner.satisfiable_item().map_err(into_err)?;
        serde_json::to_string(item).map_err(into_err)
    }

    /// Get list of selectable conditions
    ///
    /// Return `None` if the `Policy` hasn't timelocks
    #[wasm_bindgen(js_name = selectableConditions)]
    pub fn selectable_conditions(&self) -> Result<Option<Vec<JsSelectableCondition>>> {
        Ok(self
            .inner
            .selectable_conditions()
            .map_err(into_err)?
            .map(|l| l.into_iter().map(|s| s.into()).collect()))
    }

    /// Check if a `Fingerprint` is involved in the `Policy`
    #[wasm_bindgen(js_name = isFingerprintInvolved)]
    pub fn is_fingerprint_involved(&self, fingerprint: &str) -> Result<bool> {
        let fingerprint: Fingerprint = Fingerprint::from_str(fingerprint).map_err(into_err)?;
        self.inner
            .is_fingerprint_involved(&fingerprint)
            .map_err(into_err)
    }

    // TODO: add search_used_signers
}
