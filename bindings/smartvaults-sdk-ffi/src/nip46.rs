// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use nostr_ffi::{EventId, PublicKey, Timestamp, NostrConnectURI};
use smartvaults_sdk::nostr::JsonUtil;
use smartvaults_sdk::types;
use uniffi::{Object, Record};

#[derive(Record)]
pub struct NostrConnectSession {
    pub uri: Arc<NostrConnectURI>,
    pub timestamp: u64,
}

#[derive(Object)]
pub struct NostrConnectRequest {
    inner: types::NostrConnectRequest,
}

impl From<types::NostrConnectRequest> for NostrConnectRequest {
    fn from(inner: types::NostrConnectRequest) -> Self {
        Self { inner }
    }
}

#[uniffi::export]
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

    pub fn timestamp(&self) -> Arc<Timestamp> {
        Arc::new(self.inner.timestamp.into())
    }

    pub fn approved(&self) -> bool {
        self.inner.approved
    }
}
