// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::core::bdk::wallet;
use smartvaults_sdk::types;
use uniffi::{Enum, Object};

#[derive(Enum)]
pub enum AddressIndex {
    New,
    LastUnused,
    Peek { index: u32 },
}

impl From<AddressIndex> for wallet::AddressIndex {
    fn from(index: AddressIndex) -> Self {
        match index {
            AddressIndex::New => wallet::AddressIndex::New,
            AddressIndex::LastUnused => wallet::AddressIndex::LastUnused,
            AddressIndex::Peek { index } => wallet::AddressIndex::Peek(index),
        }
    }
}

#[derive(Object)]
pub struct GetAddress {
    inner: types::GetAddress,
}

impl From<types::GetAddress> for GetAddress {
    fn from(inner: types::GetAddress) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
impl GetAddress {
    pub fn address(&self) -> String {
        self.inner.address.clone().assume_checked().to_string()
    }

    pub fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }
}
