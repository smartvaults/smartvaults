// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

mod cache;
mod client;
mod error;
mod policy;
mod proposal;

mod ffi {
    // Error
    pub use crate::error::FFIError;

    // External
    pub use coinstr_core::bitcoin::Network;
    pub use coinstr_core::types::WordCount;

    // Coinstr
    pub use crate::cache::Cache;
    pub use crate::client::Coinstr;
    pub use crate::policy::Policy;
    pub use crate::proposal::Proposal;

    // UDL
    uniffi_macros::include_scaffolding!("coinstr");
}
pub use ffi::*;
