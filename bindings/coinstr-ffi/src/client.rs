// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::str::FromStr;
use std::time::Duration;

use coinstr_sdk::client;
use coinstr_sdk::core::bips::bip39::Mnemonic;
use coinstr_sdk::core::bitcoin::Network;
use coinstr_sdk::core::types::WordCount;
use coinstr_sdk::nostr::{block_on, EventId};

use crate::error::Result;

pub struct Coinstr {
    inner: client::Coinstr,
}

impl Coinstr {
    pub fn open(
        base_path: String,
        name: String,
        password: String,
        network: Network,
    ) -> Result<Self> {
        Ok(Self {
            inner: client::Coinstr::open(base_path, name, || Ok(password), network)?,
        })
    }

    pub fn generate(
        base_path: String,
        name: String,
        password: String,
        word_count: WordCount,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        Ok(Self {
            inner: client::Coinstr::generate(
                base_path,
                name,
                || Ok(password),
                word_count,
                || Ok(passphrase),
                network,
            )?,
        })
    }

    pub fn restore(
        base_path: String,
        name: String,
        password: String,
        mnemonic: String,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        let mnemonic = Mnemonic::from_str(&mnemonic)?;
        Ok(Self {
            inner: client::Coinstr::restore(
                base_path,
                name,
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
        self.inner.set_electrum_endpoint(endpoint)
    }

    pub fn electrum_endpoint(&self) -> Result<String> {
        Ok(self.inner.electrum_endpoint()?)
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
            let policy_id = self
                .inner
                .save_policy(name, description, descriptor, None)
                .await?;
            Ok(policy_id.to_hex())
        })
    }

    /* pub fn spend(
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
    } */

    pub fn approve(&self, proposal_id: String, timeout: Option<Duration>) -> Result<()> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            self.inner.approve(proposal_id, timeout).await?;
            Ok(())
        })
    }

    pub fn finalize(&self, proposal_id: String, timeout: Option<Duration>) -> Result<()> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            self.inner.finalize(proposal_id, timeout).await?;
            Ok(())
        })
    }

    pub fn sync(&self) {
        self.inner.sync();
    }
}
