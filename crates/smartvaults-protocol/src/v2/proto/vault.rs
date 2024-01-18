// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

include!(concat!(env!("OUT_DIR"), "/vault.rs"));

pub use self::vault::Object as ProtoVaultObject;
pub use self::{
    Vault as ProtoVault, VaultIdentifier as ProtoVaultIdentifier,
    VaultMetadata as ProtoVaultMetadata, VaultV1 as ProtoVaultV1,
};
