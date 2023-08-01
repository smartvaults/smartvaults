// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bdk::wallet;
use coinstr_sdk::db::model;

pub enum AddressIndex {
    New,
    LastUnused,
    Peek { index: u32 },
    Reset { index: u32 },
}

impl From<AddressIndex> for wallet::AddressIndex {
    fn from(index: AddressIndex) -> Self {
        match index {
            AddressIndex::New => wallet::AddressIndex::New,
            AddressIndex::LastUnused => wallet::AddressIndex::LastUnused,
            AddressIndex::Peek { index } => wallet::AddressIndex::Peek(index),
            AddressIndex::Reset { index } => wallet::AddressIndex::Reset(index),
        }
    }
}

pub struct GetAddress {
    inner: model::GetAddress,
}

impl From<model::GetAddress> for GetAddress {
    fn from(inner: model::GetAddress) -> Self {
        Self { inner }
    }
}

impl GetAddress {
    pub fn address(&self) -> String {
        self.inner.address.to_string()
    }

    pub fn label(&self) -> Option<String> {
        self.inner.label.clone()
    }
}
