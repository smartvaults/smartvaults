// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::db::model::GetApprovedProposal;
use nostr_ffi::{PublicKey, Timestamp};

pub struct Approval {
    inner: GetApprovedProposal,
}

impl From<GetApprovedProposal> for Approval {
    fn from(inner: GetApprovedProposal) -> Self {
        Self { inner }
    }
}

impl Approval {
    pub fn public_key(&self) -> Arc<PublicKey> {
        Arc::new(self.inner.public_key.into())
    }

    pub fn timestamp(&self) -> Arc<Timestamp> {
        Arc::new(self.inner.timestamp.into())
    }
}
