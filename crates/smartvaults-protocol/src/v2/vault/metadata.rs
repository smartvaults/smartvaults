// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Vault metadata

/// Vault metadata
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultMetadata {
    /// Name
    pub name: String,
    /// Description
    pub description: String,
}

impl VaultMetadata {
    /// New
    pub fn new<S>(name: S, description: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}
