// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use coinstr_core::bip39::Mnemonic;
use coinstr_core::bitcoin::{Address, Network};
use coinstr_core::nostr_sdk::{block_on, EventId};
use coinstr_core::types::WordCount;
use coinstr_core::{client, Amount, FeeRate};

use crate::error::Result;
use crate::Cache;

pub struct Coinstr {
    inner: client::Coinstr,
}

impl Coinstr {
    pub fn open(path: String, password: String, network: Network) -> Result<Self> {
        Ok(Self {
            inner: client::Coinstr::open(path, || Ok(password), network)?,
        })
    }

    pub fn generate(
        path: String,
        password: String,
        word_count: WordCount,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        Ok(Self {
            inner: client::Coinstr::generate(
                path,
                || Ok(password),
                word_count,
                || Ok(passphrase),
                network,
            )?,
        })
    }

    pub fn restore(
        path: String,
        password: String,
        mnemonic: String,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        let mnemonic = Mnemonic::from_str(&mnemonic)?;
        Ok(Self {
            inner: client::Coinstr::restore(
                path,
                || Ok(password),
                || Ok(mnemonic),
                || Ok(passphrase),
                network,
            )?,
        })
    }

    pub fn save(&self) -> Result<()> {
        Ok(self.inner.save()?)
    }

    pub fn check_password(&self, password: String) -> bool {
        self.inner.check_password(password)
    }

    pub fn wipe(&self) -> Result<()> {
        Ok(self.inner.wipe()?)
    }

    pub fn network(&self) -> Network {
        self.inner.network()
    }

    pub fn cache(&self) -> Arc<Cache> {
        Arc::new(self.inner.cache.clone().into())
    }

    pub fn add_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.add_relay(url, None).await?) })
    }

    pub fn connect(&self) {
        block_on(async move {
            self.inner.connect().await;
        })
    }

    pub fn add_relays_and_connect(&self, relays: Vec<String>) -> Result<()> {
        block_on(async move { Ok(self.inner.add_relays_and_connect(relays).await?) })
    }

    pub fn remove_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.remove_relay(url).await?) })
    }

    pub fn shutdown(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.clone().shutdown().await?) })
    }

    pub fn set_electrum_endpoint(&self, endpoint: String) {
        block_on(async move {
            self.inner.set_electrum_endpoint(endpoint).await;
        })
    }

    pub fn electrum_endpoint(&self) -> Result<String> {
        block_on(async move { Ok(self.inner.electrum_endpoint().await?) })
    }

    pub fn delete_policy_by_id(&self, policy_id: String, timeout: Option<Duration>) -> Result<()> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            Ok(self.inner.delete_policy_by_id(policy_id, timeout).await?)
        })
    }

    pub fn delete_proposal_by_id(
        &self,
        proposal_id: String,
        timeout: Option<Duration>,
    ) -> Result<()> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            Ok(self
                .inner
                .delete_proposal_by_id(proposal_id, timeout)
                .await?)
        })
    }

    pub fn delete_completed_proposal_by_id(
        &self,
        completed_proposal_id: String,
        timeout: Option<Duration>,
    ) -> Result<()> {
        block_on(async move {
            let completed_proposal_id = EventId::from_hex(completed_proposal_id)?;
            Ok(self
                .inner
                .delete_completed_proposal_by_id(completed_proposal_id, timeout)
                .await?)
        })
    }

    pub fn save_policy(
        &self,
        name: String,
        description: String,
        descriptor: String,
    ) -> Result<String> {
        block_on(async move {
            let (policy_id, ..) = self
                .inner
                .save_policy(name, description, descriptor)
                .await?;
            Ok(policy_id.to_hex())
        })
    }

    pub fn spend(
        &self,
        policy_id: String,
        to_address: String,
        amount: u64,
        description: String,
        target_blocks: u16,
        timeout: Option<Duration>,
    ) -> Result<String> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            let to_address = Address::from_str(&to_address)?;
            let amount = Amount::Custom(amount);
            let fee_rate = FeeRate::Custom(target_blocks as usize);
            let (proposal_id, ..) = self
                .inner
                .spend(
                    policy_id,
                    to_address,
                    amount,
                    description,
                    fee_rate,
                    timeout,
                )
                .await?;
            Ok(proposal_id.to_hex())
        })
    }

    pub fn spend_all(
        &self,
        policy_id: String,
        to_address: String,
        description: String,
        target_blocks: u16,
        timeout: Option<Duration>,
    ) -> Result<String> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            let to_address = Address::from_str(&to_address)?;
            let amount = Amount::Max;
            let fee_rate = FeeRate::Custom(target_blocks as usize);
            let (proposal_id, ..) = self
                .inner
                .spend(
                    policy_id,
                    to_address,
                    amount,
                    description,
                    fee_rate,
                    timeout,
                )
                .await?;
            Ok(proposal_id.to_hex())
        })
    }

    pub fn approve(&self, proposal_id: String, timeout: Option<Duration>) -> Result<()> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            self.inner.approve(proposal_id, timeout).await?;
            Ok(())
        })
    }

    pub fn broadcast(&self, proposal_id: String, timeout: Option<Duration>) -> Result<String> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            let txid = self.inner.broadcast(proposal_id, timeout).await?;
            Ok(txid.to_string())
        })
    }

    pub fn sync(&self) {
        self.inner.sync();
    }
}
