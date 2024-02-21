// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::ops::Deref;
use std::str::FromStr;

use smartvaults_core::miniscript::DescriptorPublicKey;
use wasm_bindgen::prelude::*;

use crate::error::{into_err, Result};

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

#[wasm_bindgen(js_class = DescriptorPublicKey)]
impl JsDescriptorPublicKey {
    pub fn parse(s: &str) -> Result<JsDescriptorPublicKey> {
        Ok(Self {
            inner: DescriptorPublicKey::from_str(s).map_err(into_err)?,
        })
    }
}
