// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_sdk::{
    config,
    nostr::{block_on, Url},
};

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
        block_on(async move { Ok(self.inner.save().await?) })
    }

    pub fn set_electrum_endpoint(&self, endpoint: String) {
        block_on(async move { self.inner.set_electrum_endpoint(Some(endpoint)).await })
    }

    pub fn electrum_endpoint(&self) -> Result<String> {
        block_on(async move { Ok(self.inner.electrum_endpoint().await?) })
    }

    pub fn set_block_explorer(&self, url: String) -> Result<()> {
        block_on(async move {
            let url = Url::parse(&url)?;
            self.inner.set_block_explorer(Some(url)).await;
            Ok(())
        })
    }

    pub fn block_explorer(&self) -> Result<String> {
        block_on(async move { Ok(self.inner.block_explorer().await?.to_string()) })
    }
}
