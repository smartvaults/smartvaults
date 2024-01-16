// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use keechain_core::bitcoin::{Address, Amount};

/// Address recipient
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Recipient {
    /// Address
    pub address: Address,
    /// Amount
    pub amount: Amount,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Destination {
    Drain(Address),
    Single(Recipient),
    Multiple(Vec<Recipient>),
}

impl Destination {
    pub fn single(address: Address, amount: Amount) -> Self {
        Self::Single(Recipient { address, amount })
    }

    /// Drain all funds to [Address]
    pub fn drain(address: Address) -> Self {
        Self::Drain(address)
    }
}
