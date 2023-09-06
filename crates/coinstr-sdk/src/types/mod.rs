// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::ops::Deref;

use coinstr_core::bdk::wallet::Balance;
use coinstr_core::bdk::LocalUtxo;
use coinstr_core::bitcoin::address::NetworkUnchecked;
use coinstr_core::bitcoin::Address;
use coinstr_core::Policy;
pub use coinstr_sdk_sqlite::model::*;
use nostr_sdk::{EventId, Timestamp};

pub mod backup;

pub use self::backup::PolicyBackup;
use crate::manager::TransactionDetails;

#[derive(Debug, Clone)]
pub struct GetPolicy {
    pub policy_id: EventId,
    pub policy: Policy,
    pub balance: Option<Balance>,
    pub last_sync: Option<Timestamp>,
}

#[derive(Debug, Clone)]
pub struct GetUtxo {
    pub utxo: LocalUtxo,
    pub label: Option<String>,
    pub frozen: bool,
}

impl Deref for GetUtxo {
    type Target = LocalUtxo;
    fn deref(&self) -> &Self::Target {
        &self.utxo
    }
}

#[derive(Debug, Clone)]
pub struct GetTransaction {
    pub policy_id: EventId,
    pub tx: TransactionDetails,
    pub label: Option<String>,
    pub block_explorer: Option<String>,
}

impl Deref for GetTransaction {
    type Target = TransactionDetails;
    fn deref(&self) -> &Self::Target {
        &self.tx
    }
}

#[derive(Debug, Clone)]
pub struct GetAddress {
    pub address: Address<NetworkUnchecked>,
    pub label: Option<String>,
}

impl Deref for GetAddress {
    type Target = Address<NetworkUnchecked>;
    fn deref(&self) -> &Self::Target {
        &self.address
    }
}

#[derive(Debug, Clone, Default)]
pub struct GetAllSigners {
    pub my: Vec<GetSigner>,
    pub contacts: Vec<GetSharedSigner>,
}
