// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use coinstr_sdk::client;
use coinstr_sdk::core::bdk::blockchain::{Blockchain, ElectrumBlockchain};
use coinstr_sdk::core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_sdk::core::bips::bip39::Mnemonic;
use coinstr_sdk::core::bitcoin::secp256k1::XOnlyPublicKey;
use coinstr_sdk::core::bitcoin::Address;
use coinstr_sdk::core::bitcoin::Network;
use coinstr_sdk::core::types::WordCount;
use coinstr_sdk::nostr::prelude::psbt::PartiallySignedTransaction;
use coinstr_sdk::nostr::{block_on, EventId};

use crate::error::Result;
use crate::{Amount, Balance, CompletedProposal, Policy, Proposal, Relay};

pub struct Coinstr {
    inner: client::Coinstr,
}

impl Coinstr {
    /// Open keychain
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

    /// Generate keychain
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

    /// Restore keychain
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

    /// Save keychain
    pub fn save(&self) -> Result<()> {
        Ok(self.inner.save()?)
    }

    /// Check keychain password
    pub fn check_password(&self, password: String) -> bool {
        self.inner.check_password(password)
    }

    // TODO: add `rename` method

    // TODO: add `change_password` method

    /// Permanent delete the keychain
    pub fn wipe(&self, password: String) -> Result<()> {
        Ok(self.inner.wipe(password)?)
    }

    pub fn clear_cache(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.clear_cache().await?) })
    }

    // TODO: add `keychain` method

    // TODO: add `keys` method

    /// Get current bitcoin network
    pub fn network(&self) -> Network {
        self.inner.network()
    }

    /// Add new relay
    pub fn add_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.add_relay(url, None).await?) })
    }

    pub fn relays(&self) -> Vec<Arc<Relay>> {
        block_on(async move {
            self.inner
                .relays()
                .await
                .into_values()
                .map(|relay| Arc::new(relay.into()))
                .collect()
        })
    }

    /// Connect relays
    pub fn connect(&self) {
        block_on(async move {
            self.inner.connect().await;
        })
    }

    pub fn default_relays(&self) -> Vec<String> {
        self.inner.default_relays()
    }

    /// Add relays
    /// Connect
    /// Rebroadcast stored events
    pub fn add_relays_and_connect(&self, relays: Vec<String>) -> Result<()> {
        block_on(async move { Ok(self.inner.add_relays_and_connect(relays).await?) })
    }

    pub fn remove_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.remove_relay(url).await?) })
    }

    /// Shutdown client
    pub fn shutdown(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.clone().shutdown().await?) })
    }

    /// Set the electrum endpoint
    pub fn set_electrum_endpoint(&self, endpoint: String) {
        self.inner.set_electrum_endpoint(endpoint)
    }

    /// Get the electrum endpoint
    pub fn electrum_endpoint(&self) -> Result<String> {
        Ok(self.inner.electrum_endpoint()?)
    }

    pub fn block_height(&self) -> u32 {
        self.inner.block_height()
    }

    // TODO: add `get_contacts` method

    /// Add new contact
    pub fn add_contact(&self, public_key: String) -> Result<()> {
        block_on(async move {
            let public_key: XOnlyPublicKey = XOnlyPublicKey::from_str(&public_key)?;
            Ok(self.inner.add_contact(public_key).await?)
        })
    }

    /// Remove contact
    pub fn remove_contact(&self, public_key: String) -> Result<()> {
        block_on(async move {
            let public_key: XOnlyPublicKey = XOnlyPublicKey::from_str(&public_key)?;
            Ok(self.inner.remove_contact(public_key).await?)
        })
    }

    pub fn get_policy_by_id(&self, policy_id: String) -> Result<Arc<Policy>> {
        let policy_id = EventId::from_hex(policy_id)?;
        Ok(Arc::new(self.inner.get_policy_by_id(policy_id)?.into()))
    }

    pub fn get_proposal_by_id(&self, proposal_id: String) -> Result<Proposal> {
        let proposal_id = EventId::from_hex(proposal_id)?;
        Ok(self.inner.get_proposal_by_id(proposal_id)?.1.into())
    }

    pub fn get_completed_proposal_by_id(
        &self,
        completed_proposal_id: String,
    ) -> Result<CompletedProposal> {
        let completed_proposal_id = EventId::from_hex(completed_proposal_id)?;
        Ok(self
            .inner
            .get_completed_proposal_by_id(completed_proposal_id)?
            .1
            .into())
    }

    pub fn delete_policy_by_id(&self, policy_id: String) -> Result<()> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            Ok(self.inner.delete_policy_by_id(policy_id).await?)
        })
    }

    pub fn delete_proposal_by_id(&self, proposal_id: String) -> Result<()> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            Ok(self.inner.delete_proposal_by_id(proposal_id).await?)
        })
    }

    pub fn delete_completed_proposal_by_id(&self, completed_proposal_id: String) -> Result<()> {
        block_on(async move {
            let completed_proposal_id = EventId::from_hex(completed_proposal_id)?;
            Ok(self
                .inner
                .delete_completed_proposal_by_id(completed_proposal_id)
                .await?)
        })
    }

    pub fn delete_signer_by_id(&self, signer_id: String) -> Result<()> {
        block_on(async move {
            let signer_id = EventId::from_hex(signer_id)?;
            Ok(self.inner.delete_signer_by_id(signer_id).await?)
        })
    }

    pub fn get_policies(&self) -> Result<HashMap<String, Arc<Policy>>> {
        let policies = self.inner.get_policies()?;
        Ok(policies
            .into_iter()
            .map(|(policy_id, res)| (policy_id.to_hex(), Arc::new(res.policy.into())))
            .collect())
    }

    // TODO: add `get_detailed_policies` method

    pub fn get_proposals(&self) -> Result<HashMap<String, Proposal>> {
        let proposals = self.inner.get_proposals()?;
        Ok(proposals
            .into_iter()
            .map(|(proposal_id, (_, proposal))| (proposal_id.to_hex(), proposal.into()))
            .collect())
    }

    pub fn get_completed_proposals(&self) -> Result<HashMap<String, CompletedProposal>> {
        let completed_proposals = self.inner.get_completed_proposals()?;
        Ok(completed_proposals
            .into_iter()
            .map(|(proposal_id, (_, proposal))| (proposal_id.to_hex(), proposal.into()))
            .collect())
    }

    pub fn spend(
        &self,
        policy_id: String,
        to_address: String,
        amount: Arc<Amount>,
        description: String,
        target_blocks: u16,
    ) -> Result<String> {
        block_on(async move {
            let endpoint: String = self.inner.electrum_endpoint()?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            let fee_rate = blockchain.estimate_fee(target_blocks as usize)?;

            let policy_id = EventId::from_hex(policy_id)?;
            let to_address = Address::from_str(&to_address)?;
            let (proposal_id, ..) = self
                .inner
                .spend(policy_id, to_address, amount.inner(), description, fee_rate)
                .await?;
            Ok(proposal_id.to_hex())
        })
    }

    pub fn self_transfer(
        &self,
        from_policy_id: String,
        to_policy_id: String,
        amount: Arc<Amount>,
        target_blocks: u16,
    ) -> Result<String> {
        block_on(async move {
            let endpoint: String = self.inner.electrum_endpoint()?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            let fee_rate = blockchain.estimate_fee(target_blocks as usize)?;

            let from_policy_id = EventId::from_hex(from_policy_id)?;
            let to_policy_id = EventId::from_hex(to_policy_id)?;
            let (proposal_id, ..) = self
                .inner
                .self_transfer(from_policy_id, to_policy_id, amount.inner(), fee_rate)
                .await?;
            Ok(proposal_id.to_hex())
        })
    }

    pub fn approve(&self, proposal_id: String) -> Result<String> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            let (approval_id, ..) = self.inner.approve(proposal_id).await?;
            Ok(approval_id.to_hex())
        })
    }

    pub fn approve_with_signed_psbt(
        &self,
        proposal_id: String,
        signed_psbt: String,
    ) -> Result<String> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            let signed_psbt = PartiallySignedTransaction::from_str(&signed_psbt)?;
            let (approval_id, ..) = self
                .inner
                .approve_with_signed_psbt(proposal_id, signed_psbt)
                .await?;
            Ok(approval_id.to_hex())
        })
    }

    pub fn finalize(&self, proposal_id: String) -> Result<CompletedProposal> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            Ok(self.inner.finalize(proposal_id).await?.into())
        })
    }

    pub fn rebroadcast_all_events(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.rebroadcast_all_events().await?) })
    }

    pub fn republish_shared_key_for_policy(&self, policy_id: String) -> Result<()> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            Ok(self
                .inner
                .republish_shared_key_for_policy(policy_id)
                .await?)
        })
    }

    pub fn get_balance(&self, policy_id: String) -> Result<Option<Arc<Balance>>> {
        let policy_id = EventId::from_hex(policy_id)?;
        Ok(self
            .inner
            .get_balance(policy_id)
            .map(|b| Arc::new(b.into())))
    }

    pub fn get_total_balance(&self) -> Result<Arc<Balance>> {
        Ok(Arc::new(self.inner.get_total_balance()?.into()))
    }

    pub fn get_last_unused_address(&self, policy_id: String) -> Result<Option<String>> {
        let policy_id = EventId::from_hex(policy_id)?;
        Ok(self
            .inner
            .get_last_unused_address(policy_id)
            .map(|a| a.to_string()))
    }

    pub fn sync(&self) {
        self.inner.sync();
    }
}
