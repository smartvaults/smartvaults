// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::fmt;
use std::ops::Deref;

use smartvaults_sdk::types::GetPolicy;
use smartvaults_sdk::util;

mod balances;
mod breadcrumb;
mod dashboard;
mod fee_selector;
mod policy_tree;
mod proposals_list;
mod transactions_list;
mod utxo_selector;

pub use self::balances::Balances;
pub use self::dashboard::Dashboard;
pub use self::fee_selector::FeeSelector;
pub use self::policy_tree::PolicyTree;
pub use self::proposals_list::{CompletedProposalsList, PendingProposalsList};
pub use self::transactions_list::TransactionsList;
pub use self::utxo_selector::UtxoSelector;

#[derive(Debug, Clone, Eq)]
pub struct PolicyPicLisk {
    inner: GetPolicy,
}

impl PartialEq for PolicyPicLisk {
    fn eq(&self, other: &Self) -> bool {
        self.inner.policy_id == other.inner.policy_id
    }
}

impl fmt::Display for PolicyPicLisk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} - #{}",
            self.inner.policy.name,
            util::cut_event_id(self.inner.policy_id)
        )
    }
}

impl Deref for PolicyPicLisk {
    type Target = GetPolicy;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl From<GetPolicy> for PolicyPicLisk {
    fn from(inner: GetPolicy) -> Self {
        Self { inner }
    }
}
