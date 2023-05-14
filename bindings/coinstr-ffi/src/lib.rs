// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

mod client;
mod error;
mod policy;
mod proposal;

use self::error::Result;

pub fn get_keychains_list(base_path: String) -> Result<Vec<String>> {
    Ok(coinstr_core::util::dir::get_keychains_list(base_path)?)
}

mod ffi {
    // Error
    pub use crate::error::FFIError;

    // External
    pub use coinstr_core::bitcoin::Network;
    pub use coinstr_core::types::WordCount;

    // Namespace
    pub use crate::get_keychains_list;

    // Coinstr
    pub use crate::client::Coinstr;
    pub use crate::policy::Policy;
    pub use crate::proposal::Proposal;

    // UDL
    uniffi_macros::include_scaffolding!("coinstr");
}
pub use ffi::*;
