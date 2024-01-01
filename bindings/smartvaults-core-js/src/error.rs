// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use wasm_bindgen::JsValue;

pub type Result<T, E = JsValue> = core::result::Result<T, E>;

#[inline]
pub fn into_err<E>(error: E) -> JsValue
where
    E: std::error::Error,
{
    JsValue::from_str(&error.to_string())
}
