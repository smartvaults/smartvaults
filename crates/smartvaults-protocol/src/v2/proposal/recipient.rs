// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Proposals recipient (address + amount)

use smartvaults_core::bitcoin::Address;

use crate::v2::proto::proposal::ProtoRecipient;

/// Address recipient
pub struct Recipient {
    /// Address
    pub address: Address,
    /// Amount in SAT
    pub amount: u64,
}

impl From<&Recipient> for ProtoRecipient {
    fn from(recipient: &Recipient) -> Self {
        ProtoRecipient {
            address: recipient.address.to_string(),
            amount: recipient.amount,
        }
    }
}
