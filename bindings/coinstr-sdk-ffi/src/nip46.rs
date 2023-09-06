// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::types;
use nostr_sdk_ffi::{EventId, PublicKey};

use crate::NostrConnectURI;

pub struct NostrConnectSession {
    pub uri: Arc<NostrConnectURI>,
    pub timestamp: u64,
}

pub struct NostrConnectRequest {
    inner: types::NostrConnectRequest,
}

impl From<types::NostrConnectRequest> for NostrConnectRequest {
    fn from(inner: types::NostrConnectRequest) -> Self {
        Self { inner }
    }
}

impl NostrConnectRequest {
    pub fn event_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.event_id.into())
    }

    pub fn app_public_key(&self) -> Arc<PublicKey> {
        Arc::new(self.inner.app_public_key.into())
    }

    pub fn message(&self) -> String {
        self.inner.message.as_json()
    }

    pub fn timestamp(&self) -> u64 {
        self.inner.timestamp.as_u64()
    }

    pub fn approved(&self) -> bool {
        self.inner.approved
    }
}
