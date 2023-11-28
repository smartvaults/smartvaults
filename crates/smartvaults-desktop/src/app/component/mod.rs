// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::fmt;
use std::ops::Deref;

use smartvaults_sdk::types::GetPolicy;
use smartvaults_sdk::util;

mod activity;
mod balances;
mod breadcrumb;
mod dashboard;
mod fee_selector;
mod policy_tree;
mod utxo_selector;

pub use self::activity::{Activity, CompletedProposalsList};
pub use self::balances::Balances;
pub use self::dashboard::Dashboard;
pub use self::fee_selector::FeeSelector;
pub use self::policy_tree::PolicyTree;
pub use self::utxo_selector::UtxoSelector;

#[derive(Debug, Clone, Eq)]
pub struct PolicyPickList {
    inner: GetPolicy,
}

impl PartialEq for PolicyPickList {
    fn eq(&self, other: &Self) -> bool {
        self.inner.policy_id == other.inner.policy_id
    }
}

impl fmt::Display for PolicyPickList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - #{}",
            self.inner.policy.name,
            util::cut_event_id(self.inner.policy_id)
        )
    }
}

impl Deref for PolicyPickList {
    type Target = GetPolicy;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<GetPolicy> for PolicyPickList {
    fn from(inner: GetPolicy) -> Self {
        Self { inner }
    }
}
