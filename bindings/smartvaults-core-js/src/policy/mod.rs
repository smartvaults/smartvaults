// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use core::ops::Deref;

use smartvaults_core::policy::Policy;
use wasm_bindgen::prelude::*;

use crate::error::{into_err, Result};
use crate::network::JsNetwork;

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
    #[wasm_bindgen(js_name = fromDescriptor)]
    pub fn from_descriptor(
        name: String,
        description: String,
        descriptor: String,
        network: JsNetwork,
    ) -> Result<JsPolicy> {
        Ok(Self {
            inner: Policy::from_descriptor(name, description, descriptor, network.into())
                .map_err(into_err)?,
        })
    }

    #[wasm_bindgen(js_name = fromMiniscript)]
    pub fn from_miniscript(
        name: String,
        description: String,
        policy: String,
        network: JsNetwork,
    ) -> Result<JsPolicy> {
        Ok(Self {
            inner: Policy::from_policy(name, description, policy, network.into())
                .map_err(into_err)?,
        })
    }

    #[wasm_bindgen(js_name = fromDescOrMiniscript)]
    pub fn from_desc_or_miniscript(
        name: String,
        description: String,
        desc_or_miniscript: String,
        network: JsNetwork,
    ) -> Result<JsPolicy> {
        Ok(Self {
            inner: Policy::from_desc_or_policy(
                name,
                description,
                desc_or_miniscript,
                network.into(),
            )
            .map_err(into_err)?,
        })
    }

    pub fn descriptor(&self) -> String {
        self.inner.descriptor().to_string()
    }
}
