// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::sync::Arc;

use coinstr_sdk::core::secp256k1::XOnlyPublicKey;
use coinstr_sdk::nostr;
use nostr_ffi::PublicKey;

use crate::Metadata;

pub struct GetContact {
    public_key: XOnlyPublicKey,
    metadata: nostr::Metadata,
}

impl From<(XOnlyPublicKey, nostr::Metadata)> for GetContact {
    fn from(value: (XOnlyPublicKey, nostr::Metadata)) -> Self {
        Self {
            public_key: value.0,
            metadata: value.1,
        }
    }
}

impl GetContact {
    pub fn public_key(&self) -> Arc<PublicKey> {
        Arc::new(self.public_key.into())
    }

    pub fn metadata(&self) -> Arc<Metadata> {
        Arc::new(self.metadata.clone().into())
    }
}
