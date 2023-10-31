// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

#![allow(clippy::module_inception)]

pub mod vault {
    include!(concat!(env!("OUT_DIR"), "/vault.rs"));

    pub use self::vault::Object as ProtoVaultObject;
    pub use self::{Vault as ProtoVault, VaultV1 as ProtoVaultV1};
}

pub mod wrapper {
    include!(concat!(env!("OUT_DIR"), "/wrapper.rs"));

    pub use self::wrapper::Object as ProtoWrapperObject;
    pub use self::Wrapper as ProtoWrapper;
}
