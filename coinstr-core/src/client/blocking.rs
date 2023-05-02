// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

//! Blocking Coinstr Client

use std::collections::HashMap;
use std::time::Duration;

use bdk::bitcoin::{Address, Network, Txid, XOnlyPublicKey};
use bdk::blockchain::Blockchain;
use bdk::database::MemoryDatabase;
use bdk::Wallet;
use nostr_sdk::block_on;
use nostr_sdk::{Event, EventId, Keys, Metadata, Result};

use super::{Amount, Error, FeeRate};
use crate::policy::Policy;
use crate::proposal::{ApprovedProposal, Proposal};

/// Blocking Coinstr Client
#[derive(Debug, Clone)]
pub struct CoinstrClient {
    client: super::CoinstrClient,
}

impl CoinstrClient {
    pub fn new(keys: Keys, relays: Vec<String>, network: Network) -> Result<Self, Error> {
        block_on(async {
            Ok(Self {
                client: super::CoinstrClient::new(keys, relays, network).await?,
            })
        })
    }

    pub fn wallet<S>(&self, descriptor: S) -> Result<Wallet<MemoryDatabase>, Error>
    where
        S: Into<String>,
    {
        self.client.wallet(descriptor)
    }

    pub fn get_contacts(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<XOnlyPublicKey, Metadata>, Error> {
        block_on(async { self.client.get_contacts(timeout).await })
    }

    pub fn get_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Policy, Keys), Error> {
        block_on(async { self.client.get_policy_by_id(policy_id, timeout).await })
    }

    pub fn get_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Proposal, EventId, Keys), Error> {
        block_on(async { self.client.get_proposal_by_id(proposal_id, timeout).await })
    }

    pub fn delete_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        block_on(async { self.client.delete_policy_by_id(policy_id, timeout).await })
    }

    pub fn delete_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        block_on(async {
            self.client
                .delete_proposal_by_id(proposal_id, timeout)
                .await
        })
    }

    pub fn get_policies(&self, timeout: Option<Duration>) -> Result<Vec<(EventId, Policy)>, Error> {
        block_on(async { self.client.get_policies(timeout).await })
    }

    pub fn get_proposals(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<(EventId, Proposal, EventId)>, Error> {
        block_on(async { self.client.get_proposals(timeout).await })
    }

    pub fn save_policy<S>(
        &self,
        name: S,
        description: S,
        descriptor: S,
    ) -> Result<(EventId, Policy), Error>
    where
        S: Into<String>,
    {
        block_on(async { self.client.save_policy(name, description, descriptor).await })
    }

    /// Make a spending proposal
    #[allow(clippy::too_many_arguments)]
    pub fn spend<S, B>(
        &self,
        policy_id: EventId,
        to_address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        blockchain: &B,
        timeout: Option<Duration>,
    ) -> Result<(EventId, Proposal), Error>
    where
        S: Into<String>,
        B: Blockchain,
    {
        block_on(async {
            self.client
                .spend(
                    policy_id,
                    to_address,
                    amount,
                    description,
                    fee_rate,
                    blockchain,
                    timeout,
                )
                .await
        })
    }

    pub fn approve(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Event, ApprovedProposal), Error> {
        block_on(async { self.client.approve(proposal_id, timeout).await })
    }

    pub fn broadcast<B>(
        &self,
        proposal_id: EventId,
        blockchain: &B,
        timeout: Option<Duration>,
    ) -> Result<Txid, Error>
    where
        B: Blockchain,
    {
        block_on(async {
            self.client
                .broadcast(proposal_id, blockchain, timeout)
                .await
        })
    }
}
