// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//! Blocking Coinstr Client

use std::collections::HashMap;
use std::time::Duration;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, Txid, XOnlyPublicKey};
use bdk::blockchain::Blockchain;
use bdk::database::MemoryDatabase;
use bdk::Wallet;
use nostr_sdk::block_on;
use nostr_sdk::{EventId, Keys, Metadata, Result};

use crate::policy::Policy;
use crate::proposal::SpendingProposal;

/// Blocking Coinstr Client
pub struct CoinstrClient {
    client: super::CoinstrClient,
}

impl CoinstrClient {
    pub fn new(keys: Keys, relays: Vec<String>, network: Network) -> Result<Self> {
        block_on(async {
            Ok(Self {
                client: super::CoinstrClient::new(keys, relays, network).await?,
            })
        })
    }

    pub fn wallet<S>(&self, descriptor: S) -> Result<Wallet<MemoryDatabase>>
    where
        S: Into<String>,
    {
        self.client.wallet(descriptor)
    }

    pub fn get_contacts(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<XOnlyPublicKey, Metadata>> {
        block_on(async { self.client.get_contacts(timeout).await })
    }

    pub fn get_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Policy, Keys)> {
        block_on(async { self.client.get_policy_by_id(policy_id, timeout).await })
    }

    pub fn get_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(SpendingProposal, EventId, Keys)> {
        block_on(async { self.client.get_proposal_by_id(proposal_id, timeout).await })
    }

    pub fn get_signed_psbts_by_proposal_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(PartiallySignedTransaction, Vec<PartiallySignedTransaction>)> {
        block_on(async {
            self.client
                .get_signed_psbts_by_proposal_id(proposal_id, timeout)
                .await
        })
    }

    pub fn delete_policy_by_id(&self, policy_id: EventId, timeout: Option<Duration>) -> Result<()> {
        block_on(async { self.client.delete_policy_by_id(policy_id, timeout).await })
    }

    pub fn delete_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<()> {
        block_on(async {
            self.client
                .delete_proposal_by_id(proposal_id, timeout)
                .await
        })
    }

    pub fn get_policies(&self, timeout: Option<Duration>) -> Result<Vec<(EventId, Policy)>> {
        block_on(async { self.client.get_policies(timeout).await })
    }

    pub fn get_proposals(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<(EventId, SpendingProposal, EventId)>> {
        block_on(async { self.client.get_proposals(timeout).await })
    }

    pub fn save_policy<S>(&self, name: S, description: S, descriptor: S) -> Result<EventId>
    where
        S: Into<String>,
    {
        block_on(async { self.client.save_policy(name, description, descriptor).await })
    }

    /// Make a spending proposal
    pub fn spend<S>(
        &self,
        policy_id: EventId,
        to_address: Address,
        amount: u64,
        memo: S,
        blockchain: impl Blockchain,
        timeout: Option<Duration>,
    ) -> Result<EventId>
    where
        S: Into<String>,
    {
        block_on(async {
            self.client
                .spend(policy_id, to_address, amount, memo, blockchain, timeout)
                .await
        })
    }

    pub fn approve(&self, proposal_id: EventId, timeout: Option<Duration>) -> Result<EventId> {
        block_on(async { self.client.approve(proposal_id, timeout).await })
    }

    pub fn broadcast(
        &self,
        proposal_id: EventId,
        blockchain: impl Blockchain,
        timeout: Option<Duration>,
    ) -> Result<Txid> {
        block_on(async {
            self.client
                .broadcast(proposal_id, blockchain, timeout)
                .await
        })
    }
}
