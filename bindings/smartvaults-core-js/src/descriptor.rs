// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::str::FromStr;

use smartvaults_core::miniscript::{Descriptor, DescriptorPublicKey};
use wasm_bindgen::prelude::*;

use crate::error::{into_err, Result};

#[wasm_bindgen(js_name = Descriptor)]
pub struct JsDescriptor {
    inner: Descriptor<String>,
}

impl Deref for JsDescriptor {
    type Target = Descriptor<String>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<Descriptor<String>> for JsDescriptor {
    fn from(inner: Descriptor<String>) -> Self {
        Self { inner }
    }
}

#[wasm_bindgen(js_class = Descriptor)]
impl JsDescriptor {
    pub fn parse(s: &str) -> Result<JsDescriptor> {
        Ok(Self {
            inner: Descriptor::from_str(s).map_err(into_err)?,
        })
    }

    #[wasm_bindgen(js_name = asString)]
    pub fn as_string(&self) -> String {
        self.inner.to_string()
    }
}

#[derive(Clone)]
#[wasm_bindgen(js_name = DescriptorPublicKey)]
pub struct JsDescriptorPublicKey {
    inner: DescriptorPublicKey,
}

impl Deref for JsDescriptorPublicKey {
    type Target = DescriptorPublicKey;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<DescriptorPublicKey> for JsDescriptorPublicKey {
    fn from(inner: DescriptorPublicKey) -> Self {
        Self { inner }
    }
}

impl From<JsDescriptorPublicKey> for DescriptorPublicKey {
    fn from(value: JsDescriptorPublicKey) -> Self {
        value.inner
    }
}

#[wasm_bindgen(js_class = DescriptorPublicKey)]
impl JsDescriptorPublicKey {
    pub fn parse(s: &str) -> Result<JsDescriptorPublicKey> {
        Ok(Self {
            inner: DescriptorPublicKey::from_str(s).map_err(into_err)?,
        })
    }

    #[wasm_bindgen(js_name = asString)]
    pub fn as_string(&self) -> String {
        self.inner.to_string()
    }
}
