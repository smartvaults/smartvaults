// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::core::bdk::wallet;

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
