// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::time::Duration;

use coinstr_core::bitcoin::Network;
use coinstr_core::client;
use coinstr_core::nostr_sdk::{block_on, EventId};

use crate::error::Result;

pub struct Coinstr {
    inner: client::Coinstr,
}

impl Coinstr {
    pub fn open(path: String, password: String, network: Network) -> Result<Self> {
        Ok(Self {
            inner: client::Coinstr::open(path, || Ok(password), network)?,
        })
    }

    pub fn approve(&self, proposal_id: String, timeout: Option<Duration>) -> Result<()> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            self.inner.approve(proposal_id, timeout).await?;
            Ok(())
        })
    }
}
