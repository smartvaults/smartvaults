// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VaultMetadata {
    pub name: String,
    pub description: String,
}

impl VaultMetadata {
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
