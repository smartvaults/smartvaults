// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::fmt;
use std::ops::Deref;

use smartvaults_sdk::types::GetVault;

mod activity;
mod balances;
mod breadcrumb;
mod dashboard;
mod fee_selector;
mod policy_tree;
mod utxo_selector;

pub use self::activity::Activity;
pub use self::balances::Balances;
pub use self::dashboard::Dashboard;
pub use self::fee_selector::FeeSelector;
pub use self::policy_tree::PolicyTree;
pub use self::utxo_selector::UtxoSelector;

#[derive(Debug, Clone, Eq)]
pub struct PolicyPickList {
    inner: GetVault,
}

impl PartialEq for PolicyPickList {
    fn eq(&self, other: &Self) -> bool {
        self.inner.vault_id == other.inner.vault_id
    }
}

impl fmt::Display for PolicyPickList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - #{}", self.inner.metadata.name, self.inner.vault_id)
    }
}

impl Deref for PolicyPickList {
    type Target = GetVault;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<GetVault> for PolicyPickList {
    fn from(inner: GetVault) -> Self {
        Self { inner }
    }
}
