// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::db::model;
use nostr_ffi::EventId;

use crate::NostrConnectURI;

pub struct NostrConnectSession {
    pub uri: Arc<NostrConnectURI>,
    pub timestamp: u64,
}

pub struct NostrConnectRequest {
    inner: model::NostrConnectRequest,
}

impl From<model::NostrConnectRequest> for NostrConnectRequest {
    fn from(inner: model::NostrConnectRequest) -> Self {
        Self { inner }
    }
}

impl NostrConnectRequest {
    pub fn event_id(&self) -> Arc<EventId> {
        Arc::new(self.inner.event_id.into())
    }

    pub fn app_public_key(&self) -> String {
        self.inner.app_public_key.to_string()
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
