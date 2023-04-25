// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap};
use std::ops::Add;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, PrivateKey, Transaction, XOnlyPublicKey};
use bdk::blockchain::Blockchain;
use bdk::database::MemoryDatabase;
use bdk::miniscript::psbt::PsbtExt;
use bdk::signer::{SignerContext, SignerOrdering, SignerWrapper};
use bdk::{KeychainKind, SignOptions, SyncOptions, TransactionDetails, Wallet};
use nostr_sdk::prelude::TagKind;
use nostr_sdk::secp256k1::SecretKey;
use nostr_sdk::{
    nips, Client, Event, EventBuilder, EventId, Filter, Keys, Metadata, Result, Tag, Timestamp,
    SECP256K1,
};

#[cfg(feature = "blocking")]
pub mod blocking;

use crate::constants::{
    APPROVED_PROPOSAL_EXPIRATION, APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, POLICY_KIND,
    SHARED_KEY_KIND, SPENDING_PROPOSAL_KIND,
};
use crate::policy::{self, Policy};
use crate::proposal::{ApprovedProposal, CompletedProposal, SpendingProposal};
use crate::{util, Amount, Encryption, EncryptionError, FeeRate};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[cfg(feature = "electrum")]
    #[error(transparent)]
    Electrum(#[from] bdk::electrum_client::Error),
    #[error(transparent)]
    Client(#[from] nostr_sdk::client::Error),
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr_sdk::event::builder::Error),
    #[error(transparent)]
    NIP04(#[from] nostr_sdk::nips::nip04::Error),
    #[error(transparent)]
    Policy(#[from] policy::Error),
    #[error(transparent)]
    Secp256k1(#[from] keechain_core::bitcoin::secp256k1::Error),
    #[error(transparent)]
    Encryption(#[from] EncryptionError),
    #[error(transparent)]
    Psbt(#[from] keechain_core::bitcoin::psbt::Error),
    #[error(transparent)]
    PsbtParse(#[from] keechain_core::bitcoin::psbt::PsbtParseError),
    #[error(transparent)]
    Util(#[from] util::Error),
    #[error("shared keys not found")]
    SharedKeysNotFound,
    #[error("policy not found")]
    PolicyNotFound,
    #[error("spending proposal not found")]
    SpendingProposalNotFound,
    #[error("approved proposal/s not found")]
    ApprovedProposalNotFound,
    #[error("impossible to finalize the PSBT: {0:?}")]
    ImpossibleToFinalizePsbt(Vec<bdk::miniscript::psbt::Error>),
    #[error("PSBT not signed")]
    PsbtNotSigned,
    #[error("wallet spending policy not found")]
    WalletSpendingPolicyNotFound,
}

struct GetApprovedProposals {
    policy_id: EventId,
    proposal: SpendingProposal,
    signed_psbts: Vec<PartiallySignedTransaction>,
    public_keys: Vec<XOnlyPublicKey>,
    approvals: Vec<XOnlyPublicKey>,
    shared_keys: Keys,
}

/// Coinstr Client
#[derive(Debug, Clone)]
pub struct CoinstrClient {
    network: Network,
    client: Client,
}

impl CoinstrClient {
    pub async fn new(keys: Keys, relays: Vec<String>, network: Network) -> Result<Self, Error> {
        let client = Client::new(&keys);
        #[cfg(not(target_arch = "wasm32"))]
        let relays = relays.iter().map(|url| (url, None)).collect();
        client.add_relays(relays).await?;
        client.connect().await;
        Ok(Self { network, client })
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub fn keys(&self) -> Keys {
        self.client.keys()
    }

    pub fn wallet<S>(&self, descriptor: S) -> Result<Wallet<MemoryDatabase>, Error>
    where
        S: Into<String>,
    {
        let db = MemoryDatabase::new();
        Ok(Wallet::new(&descriptor.into(), None, self.network, db)?)
    }

    pub async fn get_contacts(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<XOnlyPublicKey, Metadata>, Error> {
        Ok(self.client.get_contact_list_metadata(timeout).await?)
    }

    pub async fn get_shared_keys(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<EventId, Keys>, Error> {
        let keys = self.client.keys();

        let filter = Filter::new()
            .pubkey(keys.public_key())
            .kind(SHARED_KEY_KIND);
        let shared_key_events = self.client.get_events_of(vec![filter], timeout).await?;

        // Index global keys by policy id
        let mut shared_keys: HashMap<EventId, Keys> = HashMap::new();
        for event in shared_key_events.into_iter() {
            for tag in event.tags {
                if let Tag::Event(event_id, ..) = tag {
                    let content =
                        nips::nip04::decrypt(&keys.secret_key()?, &event.pubkey, &event.content)?;
                    let sk = SecretKey::from_str(&content)?;
                    let keys = Keys::new(sk);
                    shared_keys.insert(event_id, keys);
                }
            }
        }
        Ok(shared_keys)
    }

    pub async fn get_shared_key_by_policy_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<Keys, Error> {
        let keys = self.client.keys();

        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let shared_key_event = events.first().ok_or(Error::SharedKeysNotFound)?;
        let content = nips::nip04::decrypt(
            &keys.secret_key()?,
            &shared_key_event.pubkey,
            &shared_key_event.content,
        )?;
        let sk = SecretKey::from_str(&content)?;
        Ok(Keys::new(sk))
    }

    pub async fn get_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Policy, Keys), Error> {
        // Get policy event
        let filter = Filter::new().id(policy_id).kind(POLICY_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let policy_event = events.first().ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        // Decrypt the policy
        Ok((
            Policy::decrypt(&shared_keys, &policy_event.content)?,
            shared_keys,
        ))
    }

    pub async fn get_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(SpendingProposal, EventId, Keys), Error> {
        // Get proposal event
        let filter = Filter::new().id(proposal_id).kind(SPENDING_PROPOSAL_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let proposal_event = events.first().ok_or(Error::SpendingProposalNotFound)?;
        let policy_id =
            util::extract_first_event_id(proposal_event).ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        // Decrypt the spending proposal
        Ok((
            SpendingProposal::decrypt(&shared_keys, &proposal_event.content)?,
            policy_id,
            shared_keys,
        ))
    }

    async fn get_approved_proposals_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<GetApprovedProposals, Error> {
        // Get approved proposals
        let filter = Filter::new()
            .event(proposal_id)
            .kind(APPROVED_PROPOSAL_KIND);
        let approvaed_proposal_events = self.client.get_events_of(vec![filter], timeout).await?;
        let first_event = approvaed_proposal_events
            .first()
            .ok_or(Error::ApprovedProposalNotFound)?;
        let proposal_id =
            util::extract_first_event_id(first_event).ok_or(Error::ApprovedProposalNotFound)?;

        // Get global shared key
        let (proposal, policy_id, shared_keys) =
            self.get_proposal_by_id(proposal_id, timeout).await?;

        let mut psbts: Vec<PartiallySignedTransaction> = Vec::new();
        let mut public_keys = Vec::new();
        let mut approvals = Vec::new();

        for event in approvaed_proposal_events.into_iter() {
            approvals.push(event.pubkey);

            let approved_proposal = ApprovedProposal::decrypt(&shared_keys, &event.content)?;
            psbts.push(approved_proposal.psbt());

            for tag in event.tags.into_iter() {
                if let Tag::PubKey(pubkey, ..) = tag {
                    if !public_keys.contains(&pubkey) {
                        public_keys.push(pubkey);
                    }
                }
            }
        }

        Ok(GetApprovedProposals {
            policy_id,
            proposal,
            signed_psbts: psbts,
            public_keys,
            approvals,
            shared_keys,
        })
    }

    pub async fn delete_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        // Get shared key
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        // Get all events linked to the policy
        let filter = Filter::new().event(policy_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?;

        let mut ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
        ids.push(policy_id);

        let event = EventBuilder::delete::<String>(ids, None).to_event(&shared_keys)?;
        self.client.send_event(event).await?;

        Ok(())
    }

    pub async fn delete_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        // Get the proposal
        let filter = Filter::new().id(proposal_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let proposal_event = events.first().ok_or(Error::SpendingProposalNotFound)?;
        let policy_id =
            util::extract_first_event_id(proposal_event).ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        // Get all events linked to the proposal
        /* let filter = Filter::new().event(proposal_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?; */

        let ids: Vec<EventId> = vec![proposal_id];
        /* let mut ids: Vec<EventId> = vec![proposal_id];

        for event in events.into_iter() {
            if event.kind != COMPLETED_PROPOSAL_KIND {
                ids.push(event.id);
            }
        } */

        let event = EventBuilder::delete::<String>(ids, None).to_event(&shared_keys)?;
        self.client.send_event(event).await?;

        Ok(())
    }

    pub async fn get_policies(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<(EventId, Policy)>, Error> {
        let keys = self.client.keys();

        // Get policies
        let filter = Filter::new().pubkey(keys.public_key()).kind(POLICY_KIND);
        let policies_events = self.client.get_events_of(vec![filter], timeout).await?;

        // Get shared keys
        let shared_keys: HashMap<EventId, Keys> = self.get_shared_keys(timeout).await?;

        let mut policies: Vec<(EventId, Policy)> = Vec::new();

        for event in policies_events.into_iter() {
            if let Some(shared_key) = shared_keys.get(&event.id) {
                policies.push((event.id, Policy::decrypt(shared_key, &event.content)?));
            } else {
                log::error!("Shared key not found for policy {}", event.id);
            }
        }

        Ok(policies)
    }

    pub async fn get_proposals(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<(EventId, SpendingProposal, EventId)>, Error> {
        let keys = self.client.keys();

        // Get proposals
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .kind(SPENDING_PROPOSAL_KIND);
        let proposals_events = self.client.get_events_of(vec![filter], timeout).await?;

        // Get shared keys
        let shared_keys: HashMap<EventId, Keys> = self.get_shared_keys(timeout).await?;

        let mut proposals: Vec<(EventId, SpendingProposal, EventId)> = Vec::new();

        for event in proposals_events.into_iter() {
            let policy_id = util::extract_first_event_id(&event).ok_or(Error::PolicyNotFound)?;
            let shared_key: &Keys = shared_keys
                .get(&policy_id)
                .ok_or(Error::SharedKeysNotFound)?;
            proposals.push((
                event.id,
                SpendingProposal::decrypt(shared_key, &event.content)?,
                policy_id,
            ));
        }

        Ok(proposals)
    }

    pub async fn get_completed_proposals(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<(EventId, CompletedProposal, EventId)>, Error> {
        let keys = self.client.keys();

        // Get completed proposals
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .kind(COMPLETED_PROPOSAL_KIND);
        let completed_proposals_events = self.client.get_events_of(vec![filter], timeout).await?;

        // Get shared keys
        let shared_keys: HashMap<EventId, Keys> = self.get_shared_keys(timeout).await?;

        let mut proposals: Vec<(EventId, CompletedProposal, EventId)> = Vec::new();

        for event in completed_proposals_events.into_iter() {
            if !event.is_expired() {
                let policy_id = util::extract_tags_by_kind(&event, TagKind::E)
                    .get(1)
                    .map(|t| {
                        if let Tag::Event(event_id, ..) = t {
                            Some(event_id)
                        } else {
                            None
                        }
                    })
                    .ok_or(Error::PolicyNotFound)?
                    .ok_or(Error::PolicyNotFound)?;
                let shared_key: &Keys = shared_keys
                    .get(policy_id)
                    .ok_or(Error::SharedKeysNotFound)?;
                proposals.push((
                    event.id,
                    CompletedProposal::decrypt(shared_key, &event.content)?,
                    *policy_id,
                ));
            }
        }

        Ok(proposals)
    }

    pub async fn save_policy<S>(
        &self,
        name: S,
        description: S,
        descriptor: S,
    ) -> Result<(EventId, Policy), Error>
    where
        S: Into<String>,
    {
        let keys = self.client.keys();
        let descriptor = descriptor.into();

        let extracted_pubkeys = util::extract_public_keys(&descriptor)?;

        // Generate a shared key
        let shared_key = Keys::generate();
        let policy = Policy::from_desc_or_policy(name, description, descriptor)?;

        // Compose the event
        let content = policy.encrypt(&shared_key)?;
        let tags: Vec<Tag> = extracted_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        // Publish policy with `shared_key` so every owner can delete it
        let policy_event = EventBuilder::new(POLICY_KIND, content, &tags).to_event(&shared_key)?;
        let policy_id = policy_event.id;

        // Publish the shared key
        for pubkey in extracted_pubkeys.into_iter() {
            let encrypted_shared_key = nips::nip04::encrypt(
                &keys.secret_key()?,
                &pubkey,
                shared_key.secret_key()?.display_secret().to_string(),
            )?;
            let event = EventBuilder::new(
                SHARED_KEY_KIND,
                encrypted_shared_key,
                &[Tag::Event(policy_id, None, None), Tag::PubKey(pubkey, None)],
            )
            .to_event(&keys)?;
            let event_id = self.client.send_event(event).await?;
            log::info!("Published shared key for {pubkey} at event {event_id}");
        }

        // Publish the event
        self.client.send_event(policy_event).await?;

        Ok((policy_id, policy))
    }

    pub async fn build_spending_proposal<S, B>(
        &self,
        policy: &Policy,
        to_address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        blockchain: &B,
    ) -> Result<(SpendingProposal, TransactionDetails), Error>
    where
        S: Into<String>,
        B: Blockchain,
    {
        // Sync balance
        let wallet = self.wallet(policy.descriptor.to_string())?;
        #[cfg(not(target_arch = "wasm32"))]
        wallet.sync(blockchain, SyncOptions::default())?;
        #[cfg(target_arch = "wasm32")]
        wallet.sync(&blockchain, SyncOptions::default()).await?;

        // Get policies and specify which ones to use
        let wallet_policy = wallet
            .policies(KeychainKind::External)?
            .ok_or(Error::WalletSpendingPolicyNotFound)?;
        let mut path = BTreeMap::new();
        path.insert(wallet_policy.id, vec![1]);

        // Calculate fee rate
        let target_blocks: usize = fee_rate.target_blocks();
        #[cfg(not(target_arch = "wasm32"))]
        let fee_rate = blockchain.estimate_fee(target_blocks)?;
        #[cfg(target_arch = "wasm32")]
        let fee_rate = blockchain.estimate_fee(target_blocks).await?;

        // Build the PSBT
        let (psbt, details) = {
            let mut builder = wallet.build_tx();
            builder
                .policy_path(path, KeychainKind::External)
                .fee_rate(fee_rate);
            match amount {
                Amount::Max => builder.drain_wallet().drain_to(to_address.script_pubkey()),
                Amount::Custom(amount) => builder.add_recipient(to_address.script_pubkey(), amount),
            };
            builder.finish()?
        };

        let amount: u64 = details.sent.saturating_sub(details.received);
        let proposal = SpendingProposal::new(to_address, amount, description, psbt);

        Ok((proposal, details))
    }

    /// Make a spending proposal
    #[allow(clippy::too_many_arguments)]
    pub async fn spend<S, B>(
        &self,
        policy_id: EventId,
        to_address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        blockchain: &B,
        timeout: Option<Duration>,
    ) -> Result<(EventId, SpendingProposal), Error>
    where
        S: Into<String>,
        B: Blockchain,
    {
        // Get policy
        let (policy, shared_keys) = self.get_policy_by_id(policy_id, timeout).await?;

        let description: &str = &description.into();

        // Build spending proposal
        let (proposal, _details) = self
            .build_spending_proposal(
                &policy,
                to_address,
                amount,
                description,
                fee_rate,
                blockchain,
            )
            .await?;

        // Compose the event
        let extracted_pubkeys = util::extract_public_keys(policy.descriptor.to_string())?;
        let mut tags: Vec<Tag> = extracted_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(policy_id, None, None));
        let content = proposal.encrypt(&shared_keys)?;
        // Publish proposal with `shared_key` so every owner can delete it
        let event =
            EventBuilder::new(SPENDING_PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;
        let proposal_id = self.client.send_event(event).await?;

        // Send DM msg
        let sender = self.client.keys().public_key();
        let mut msg = String::from("New spending proposal:\n");
        msg.push_str(&format!(
            "- Amount: {} sat\n",
            util::format::big_number(proposal.amount)
        ));
        msg.push_str(&format!("- Description: {description}"));
        for pubkey in extracted_pubkeys.into_iter() {
            if sender != pubkey {
                self.client.send_direct_msg(pubkey, &msg).await?;
            }
        }

        Ok((proposal_id, proposal))
    }

    fn is_internal_key<S>(&self, descriptor: S) -> Result<bool, Error>
    where
        S: Into<String>,
    {
        let descriptor = descriptor.into();
        let keys = self.client.keys();
        Ok(
            descriptor.starts_with(&format!("tr({}", keys.secret_key()?.public_key(SECP256K1)))
                || descriptor.starts_with(&format!("tr({}", keys.public_key())),
        )
    }

    pub fn approve_spending_proposal(
        &self,
        policy: &Policy,
        proposal: &SpendingProposal,
    ) -> Result<ApprovedProposal, Error> {
        let keys = self.client.keys();

        // Create a BDK wallet
        let mut wallet = self.wallet(policy.descriptor.to_string())?;

        // Add the BDK signer
        let private_key = PrivateKey::new(keys.secret_key()?, self.network);
        let signer = SignerWrapper::new(
            private_key,
            SignerContext::Tap {
                is_internal_key: self.is_internal_key(policy.descriptor.to_string())?,
            },
        );

        wallet.add_signer(KeychainKind::External, SignerOrdering(0), Arc::new(signer));

        // Sign the transaction
        let mut psbt = proposal.psbt.clone();
        let _finalized = wallet.sign(&mut psbt, SignOptions::default())?;
        if psbt != proposal.psbt {
            Ok(ApprovedProposal::new(psbt))
        } else {
            Err(Error::PsbtNotSigned)
        }
    }

    pub async fn approve(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Event, ApprovedProposal), Error> {
        let keys = self.client.keys();

        // Get proposal
        let (proposal, policy_id, shared_keys) =
            self.get_proposal_by_id(proposal_id, timeout).await?;

        // Get policy id
        let (policy, _shared_keys) = self.get_policy_by_id(policy_id, timeout).await?;

        // Sign PSBT
        let approved_proposal = self.approve_spending_proposal(&policy, &proposal)?;

        // Compose the event
        let content = approved_proposal.encrypt(&shared_keys)?;
        let extracted_pubkeys = util::extract_public_keys(policy.descriptor.to_string())?;
        let mut tags: Vec<Tag> = extracted_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        tags.push(Tag::Expiration(
            Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
        ));

        let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;

        // Publish the event
        self.client.send_event(event.clone()).await?;

        Ok((event, approved_proposal))
    }

    pub fn combine_psbts(
        &self,
        base_psbt: PartiallySignedTransaction,
        signed_psbts: Vec<PartiallySignedTransaction>,
    ) -> Result<Transaction, Error> {
        let mut base_psbt = base_psbt;

        // Combine PSBTs
        for psbt in signed_psbts {
            base_psbt.combine(psbt)?;
        }

        // Finalize the transaction
        base_psbt
            .finalize_mut(SECP256K1)
            .map_err(Error::ImpossibleToFinalizePsbt)?;

        Ok(base_psbt.extract_tx())
    }

    pub async fn broadcast<B>(
        &self,
        proposal_id: EventId,
        blockchain: &B,
        timeout: Option<Duration>,
    ) -> Result<(EventId, EventId, CompletedProposal), Error>
    where
        B: Blockchain,
    {
        // Get PSBTs
        let GetApprovedProposals {
            policy_id,
            proposal,
            signed_psbts,
            public_keys,
            approvals,
            shared_keys,
        } = self
            .get_approved_proposals_by_id(proposal_id, timeout)
            .await?;

        // Combine PSBTs
        let finalized_tx = self.combine_psbts(proposal.psbt, signed_psbts)?;

        // Broadcast
        #[cfg(not(target_arch = "wasm32"))]
        blockchain.broadcast(&finalized_tx)?;
        #[cfg(target_arch = "wasm32")]
        blockchain.broadcast(&finalized_tx).await?;

        // Build the broadcasted proposal
        let completed_proposal =
            CompletedProposal::new(finalized_tx.txid(), proposal.description, approvals);

        // Compose the event
        let content = completed_proposal.encrypt(&shared_keys)?;
        let mut tags: Vec<Tag> = public_keys.iter().map(|p| Tag::PubKey(*p, None)).collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        let event =
            EventBuilder::new(COMPLETED_PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;

        // Publish the event
        let event_id = self.client.send_event(event).await?;

        // Delete the proposal
        if let Err(e) = self.delete_proposal_by_id(proposal_id, timeout).await {
            log::error!("Impossibe to delete proposal {proposal_id}: {e}");
        }

        Ok((event_id, policy_id, completed_proposal))
    }

    pub fn inner(&self) -> Client {
        self.client.clone()
    }
}

#[cfg(feature = "electrum")]
#[cfg(test)]
mod test {
    use bdk::blockchain::ElectrumBlockchain;
    use bdk::electrum_client::Client as ElectrumClient;
    use nostr_sdk::prelude::FromSkStr;

    use super::*;

    const NETWORK: Network = Network::Testnet;
    const BITCOIN_ENDPOINT: &str = "ssl://blockstream.info:993";

    #[tokio::test]
    async fn test_spend_approve_combine() -> Result<()> {
        let descriptor = "tr(38e977f65c9d4f7adafc50d7a181a5a4fcbbce3cda2f29bd123163e21e9bf307,multi_a(2,f831caf722214748c72db4829986bd0cbb2bb8b3aeade1c959624a52a9629046,3eea9e831fefdaa8df35187a204d82edb589a36b170955ac5ca6b88340befaa0))#39a2m6vn";

        let keys_a =
            Keys::from_sk_str("1614a50390bc2c2ed7d2a68caeb3f79fb8c9ec76a7fecaa6c60ded40652ab684")?;
        let keys_b =
            Keys::from_sk_str("d5a2db059247c393d8d11bab1374b20390c1ec162aaca3782578f2bf433ebeb7")?;

        let client_a = CoinstrClient::new(keys_a, Vec::new(), NETWORK).await?;
        let client_b = CoinstrClient::new(keys_b, Vec::new(), NETWORK).await?;

        let policy = Policy::from_descriptor("Name", "Description", descriptor)?;

        // Build spending proposal
        let blockchain = ElectrumBlockchain::from(ElectrumClient::new(BITCOIN_ENDPOINT)?);
        let (proposal, _) = client_a
            .build_spending_proposal(
                &policy,
                Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78")?,
                Amount::Custom(1120),
                "Testing",
                FeeRate::default(),
                &blockchain,
            )
            .await?;

        // Sign
        let approved_proposal_a = client_a.approve_spending_proposal(&policy, &proposal)?;
        let approved_proposal_b = client_b.approve_spending_proposal(&policy, &proposal)?;

        // Combine PSBTs
        let _tx = client_b.combine_psbts(
            proposal.psbt,
            vec![approved_proposal_a.psbt(), approved_proposal_b.psbt()],
        )?;

        Ok(())
    }
}
