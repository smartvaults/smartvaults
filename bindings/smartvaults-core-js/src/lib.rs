// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

#![allow(clippy::drop_non_drop)]
#![allow(non_snake_case)]
#![allow(clippy::new_without_default)]

#[cfg(feature = "console_error_panic_hook")]
use wasm_bindgen::prelude::*;

pub mod error;
pub mod network;
pub mod policy;

/// Run some stuff when the Wasm module is instantiated.
///
/// Right now, it does the following:
///
/// * Redirect Rust panics to JavaScript console.
#[cfg(feature = "console_error_panic_hook")]
#[wasm_bindgen(start)]
pub fn start() {
    console_error_panic_hook::set_once();
}
