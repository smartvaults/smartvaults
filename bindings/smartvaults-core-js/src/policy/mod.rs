// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;
use core::str::FromStr;

use smartvaults_core::bitcoin::bip32::Fingerprint;
use smartvaults_core::policy::{Policy, PolicyPathSelector};
use smartvaults_core::SelectableCondition;
use wasm_bindgen::prelude::*;

pub mod template;

use self::template::{JsPolicyTemplate, JsPolicyTemplateType};
use crate::descriptor::JsDescriptor;
use crate::error::{into_err, Result};
use crate::network::JsNetwork;
use crate::signer::JsCoreSigner;

#[wasm_bindgen(js_name = PolicyPathSelectedItem)]
pub struct JsPolicyPathSelectedItem {
    #[wasm_bindgen(getter_with_clone)]
    pub path: String,
    /// Sub paths indexes
    #[wasm_bindgen(getter_with_clone)]
    pub indexes: Vec<usize>,
}

impl From<(String, Vec<usize>)> for JsPolicyPathSelectedItem {
    fn from((path, indexes): (String, Vec<usize>)) -> Self {
        Self { path, indexes }
    }
}

#[wasm_bindgen(js_name = PolicyPathMissingToSelectedItem)]
pub struct JsPolicyPathMissingToSelectedItem {
    #[wasm_bindgen(getter_with_clone)]
    pub path: String,
    #[wasm_bindgen(getter_with_clone)]
    pub sub_paths: Vec<String>,
}

impl From<(String, Vec<String>)> for JsPolicyPathMissingToSelectedItem {
    fn from((path, sub_paths): (String, Vec<String>)) -> Self {
        Self { path, sub_paths }
    }
}

#[wasm_bindgen(js_name = PolicyPathSelector)]
pub struct JsPolicyPathSelector {
    inner: PolicyPathSelector,
}

impl From<PolicyPathSelector> for JsPolicyPathSelector {
    fn from(inner: PolicyPathSelector) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = PolicyPathSelector)]
impl JsPolicyPathSelector {
    #[wasm_bindgen(js_name = isComplete)]
    pub fn is_complete(&self) -> bool {
        self.inner.is_complete()
    }

    #[wasm_bindgen(js_name = isPartial)]
    pub fn is_partial(&self) -> bool {
        self.inner.is_partial()
    }

    /// Selected path
    #[wasm_bindgen(js_name = selectedPath)]
    pub fn selected_path(&self) -> Vec<JsPolicyPathSelectedItem> {
        self.inner
            .selected_path()
            .clone()
            .into_iter()
            .map(|i| i.into())
            .collect()
    }

    /// Missing paths to select
    #[wasm_bindgen(js_name = missingToSelect)]
    pub fn missing_to_select(&self) -> Vec<JsPolicyPathMissingToSelectedItem> {
        self.inner
            .missing_to_select()
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|i| i.into())
            .collect()
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

    /// Construct `Policy` from miniscripto policy
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
    pub fn from_template(template: &JsPolicyTemplate, network: JsNetwork) -> Result<JsPolicy> {
        Ok(Self {
            inner: Policy::from_template(template.deref().clone(), network.into())
                .map_err(into_err)?,
        })
    }

    pub fn descriptor(&self) -> JsDescriptor {
        self.inner.descriptor().into()
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

    #[wasm_bindgen(js_name = getPolicyPathFromSigner)]
    pub fn get_policy_path_from_signer(
        &self,
        signer: &JsCoreSigner,
    ) -> Result<Option<JsPolicyPathSelector>> {
        Ok(self
            .inner
            .get_policy_path_from_signer(signer.deref())
            .map_err(into_err)?
            .map(|s| s.into()))
    }

    // TODO: add get_policy_paths_from_signers

    /// Check if `Policy` match match any template
    #[wasm_bindgen(js_name = templateMatch)]
    pub fn template_match(&self) -> Result<Option<JsPolicyTemplateType>> {
        Ok(self
            .inner
            .template_match()
            .map_err(into_err)?
            .map(|t| t.into()))
    }

    // TODO: add spend
}
