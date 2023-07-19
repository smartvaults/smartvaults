// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::{config, nostr::Url};

use crate::error::Result;

pub struct Config {
    inner: config::Config,
}

impl From<config::Config> for Config {
    fn from(inner: config::Config) -> Self {
        Self { inner }
    }
}

impl Config {
    pub fn save(&self) -> Result<()> {
        Ok(self.inner.save()?)
    }

    pub fn set_electrum_endpoint(&self, endpoint: String) {
        self.inner.set_electrum_endpoint(Some(endpoint))
    }

    pub fn electrum_endpoint(&self) -> Result<String> {
        Ok(self.inner.electrum_endpoint()?)
    }

    pub fn set_block_explorer(&self, url: String) -> Result<()> {
        let url = Url::parse(&url)?;
        self.inner.set_block_explorer(Some(url));
        Ok(())
    }

    pub fn block_explorer(&self) -> Result<String> {
        Ok(self.inner.block_explorer()?.to_string())
    }
}
