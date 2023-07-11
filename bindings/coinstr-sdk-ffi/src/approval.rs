// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::db::model::GetApprovedProposalResult;

pub struct Approval {
    inner: GetApprovedProposalResult,
}

impl From<GetApprovedProposalResult> for Approval {
    fn from(inner: GetApprovedProposalResult) -> Self {
        Self { inner }
    }
}

impl Approval {
    pub fn public_key(&self) -> String {
        self.inner.public_key.to_string()
    }

    pub fn timestamp(&self) -> u64 {
        self.inner.timestamp.as_u64()
    }
}
