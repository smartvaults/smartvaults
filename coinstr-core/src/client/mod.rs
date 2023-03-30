// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap};
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, PrivateKey, Txid, XOnlyPublicKey};
use bdk::blockchain::Blockchain;
use bdk::database::MemoryDatabase;
use bdk::miniscript::psbt::PsbtExt;
use bdk::signer::{SignerContext, SignerOrdering, SignerWrapper};
use bdk::{KeychainKind, SignOptions, SyncOptions, Wallet};
use nostr_sdk::secp256k1::SecretKey;
use nostr_sdk::{
    nips, Client, EventBuilder, EventId, Filter, Keys, Metadata, Result, Tag, SECP256K1,
};

#[cfg(feature = "blocking")]
pub mod blocking;

use crate::constants::{
    APPROVED_PROPOSAL_KIND, POLICY_KIND, SHARED_KEY_KIND, SPENDING_PROPOSAL_KIND,
};
use crate::policy::Policy;
use crate::proposal::SpendingProposal;
use crate::util;

/// Coinstr Client
pub struct CoinstrClient {
    network: Network,
    client: Client,
}

impl CoinstrClient {
    pub async fn new(keys: Keys, relays: Vec<String>, network: Network) -> Result<Self> {
        let client = Client::new(&keys);
        #[cfg(not(target_arch = "wasm32"))]
        let relays = relays.iter().map(|url| (url, None)).collect();
        client.add_relays(relays).await?;
        client.connect().await;
        Ok(Self { network, client })
    }

    pub fn wallet<S>(&self, descriptor: S) -> Result<Wallet<MemoryDatabase>>
    where
        S: Into<String>,
    {
        let db = MemoryDatabase::new();
        Ok(Wallet::new(&descriptor.into(), None, self.network, db)?)
    }

    pub async fn get_contacts(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<XOnlyPublicKey, Metadata>> {
        Ok(self.client.get_contact_list_metadata(timeout).await?)
    }

    async fn get_shared_keys(&self, timeout: Option<Duration>) -> Result<HashMap<EventId, Keys>> {
        let keys = self.client.keys();

        let filter = Filter::new()
            .pubkey(keys.public_key())
            .kind(SHARED_KEY_KIND);
        let global_shared_key_events = self.client.get_events_of(vec![filter], timeout).await?;

        // Index global keys by policy id
        let mut shared_keys: HashMap<EventId, Keys> = HashMap::new();
        for event in global_shared_key_events.into_iter() {
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

    pub async fn get_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Policy, Keys)> {
        let keys = self.client.keys();

        // Get policy event
        let filter = Filter::new().id(policy_id).kind(POLICY_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let policy_event = events.first().expect("Policy not found");

        // Get global shared key
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let global_shared_key_event = events.first().expect("Shared key not found");
        let content = nips::nip04::decrypt(
            &keys.secret_key()?,
            &global_shared_key_event.pubkey,
            &global_shared_key_event.content,
        )?;
        let sk = SecretKey::from_str(&content)?;
        let shared_keys = Keys::new(sk);

        // Decrypt and deserialize the policy
        let content = nips::nip04::decrypt(
            &shared_keys.secret_key()?,
            &shared_keys.public_key(),
            &policy_event.content,
        )?;
        Ok((Policy::from_json(content)?, shared_keys))
    }

    pub async fn get_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(SpendingProposal, EventId, Keys)> {
        let keys = self.client.keys();

        // Get proposal event
        let filter = Filter::new().id(proposal_id).kind(SPENDING_PROPOSAL_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let proposal_event = events.first().expect("Spending proposal not found");
        let policy_id = util::extract_first_event_id(proposal_event).expect("Policy id not found");

        // Get global shared key
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let global_shared_key_event = events.first().expect("Shared key not found");
        let content = nips::nip04::decrypt(
            &keys.secret_key()?,
            &global_shared_key_event.pubkey,
            &global_shared_key_event.content,
        )?;
        let sk = SecretKey::from_str(&content)?;
        let shared_keys = Keys::new(sk);

        // Decrypt and deserialize the spending proposal
        let content = nips::nip04::decrypt(
            &shared_keys.secret_key()?,
            &shared_keys.public_key(),
            &proposal_event.content,
        )?;
        Ok((
            SpendingProposal::from_json(content)?,
            policy_id,
            shared_keys,
        ))
    }

    pub async fn get_signed_psbts_by_proposal_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(PartiallySignedTransaction, Vec<PartiallySignedTransaction>)> {
        // Get approved proposals
        let filter = Filter::new()
            .event(proposal_id)
            .kind(APPROVED_PROPOSAL_KIND);
        let proposals_events = self.client.get_events_of(vec![filter], timeout).await?;
        let first_event = proposals_events
            .first()
            .expect("Approved proposals not found");
        let proposal_id = util::extract_first_event_id(first_event).expect("Proposal id not found");

        // Get global shared key
        let (proposal, _, shared_keys) = self.get_proposal_by_id(proposal_id, timeout).await?;

        let mut psbts: Vec<PartiallySignedTransaction> = Vec::new();

        for event in proposals_events.into_iter() {
            let content = nips::nip04::decrypt(
                &shared_keys.secret_key()?,
                &shared_keys.public_key(),
                &event.content,
            )?;
            psbts.push(PartiallySignedTransaction::from_str(&content)?);
        }

        Ok((proposal.psbt, psbts))
    }

    pub async fn delete_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<()> {
        let keys = self.client.keys();

        // Get global shared key
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let global_shared_key_event = events.first().expect("Shared key not found");
        let content = nips::nip04::decrypt(
            &keys.secret_key()?,
            &global_shared_key_event.pubkey,
            &global_shared_key_event.content,
        )?;
        let sk = SecretKey::from_str(&content)?;
        let shared_keys = Keys::new(sk);

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
    ) -> Result<()> {
        let keys = self.client.keys();

        // Get the proposal
        let filter = Filter::new().id(proposal_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let proposal_event = events.first().expect("Spending proposal not found");
        let policy_id = util::extract_first_event_id(proposal_event).expect("Policy id not found");

        // Get global shared key
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let global_shared_key_event = events.first().expect("Shared key not found");
        let content = nips::nip04::decrypt(
            &keys.secret_key()?,
            &global_shared_key_event.pubkey,
            &global_shared_key_event.content,
        )?;
        let sk = SecretKey::from_str(&content)?;
        let shared_keys = Keys::new(sk);

        // Get all events linked to the proposal
        let filter = Filter::new().event(proposal_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?;

        let mut ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
        ids.push(proposal_id);

        let event = EventBuilder::delete::<String>(ids, None).to_event(&shared_keys)?;
        self.client.send_event(event).await?;

        Ok(())
    }

    pub async fn get_policies(&self, timeout: Option<Duration>) -> Result<Vec<(EventId, Policy)>> {
        let keys = self.client.keys();

        // Get policies
        let filter = Filter::new().pubkey(keys.public_key()).kind(POLICY_KIND);
        let policies_events = self.client.get_events_of(vec![filter], timeout).await?;

        // Get shared keys
        let shared_keys: HashMap<EventId, Keys> = self.get_shared_keys(timeout).await?;

        let mut policies: Vec<(EventId, Policy)> = Vec::new();

        for event in policies_events.into_iter() {
            let global_key = shared_keys.get(&event.id).expect("Global key not found");
            let content = nips::nip04::decrypt(
                &global_key.secret_key()?,
                &global_key.public_key(),
                &event.content,
            )?;
            policies.push((event.id, Policy::from_json(&content)?));
        }

        Ok(policies)
    }

    pub async fn get_proposals(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<(EventId, SpendingProposal, EventId)>> {
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
            let policy_id = util::extract_first_event_id(&event).expect("Policy id not found");
            let global_key: &Keys = shared_keys.get(&policy_id).expect("Global key not found");

            let content = nips::nip04::decrypt(
                &global_key.secret_key()?,
                &global_key.public_key(),
                &event.content,
            )?;

            proposals.push((event.id, SpendingProposal::from_json(&content)?, policy_id));
        }

        Ok(proposals)
    }

    pub async fn save_policy<S>(&self, name: S, description: S, descriptor: S) -> Result<EventId>
    where
        S: Into<String>,
    {
        let keys = self.client.keys();
        let descriptor = descriptor.into();

        let extracted_pubkeys = util::extract_public_keys(&descriptor)?;

        // Generate a shared key
        let shared_key = Keys::generate();
        let policy = Policy::from_desc_or_policy(name, description, descriptor)?;
        let content = nips::nip04::encrypt(
            &shared_key.secret_key()?,
            &shared_key.public_key(),
            policy.as_json(),
        )?;
        let tags: Vec<Tag> = extracted_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        // Publish policy with `shared_key` so every owner can delete it
        let policy_event = EventBuilder::new(POLICY_KIND, content, &tags).to_event(&shared_key)?;
        let policy_id = self.client.send_event(policy_event).await?;

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

        Ok(policy_id)
    }

    /// Make a spending proposal
    pub async fn spend<S>(
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
        // Get policy
        let (policy, shared_keys) = self.get_policy_by_id(policy_id, timeout).await?;

        // Sync balance
        let wallet = self.wallet(policy.descriptor.to_string())?;
        #[cfg(not(target_arch = "wasm32"))]
        wallet.sync(&blockchain, SyncOptions::default())?;
        #[cfg(target_arch = "wasm32")]
        wallet.sync(&blockchain, SyncOptions::default()).await?;

        // Get policies and specify which ones to use
        let wallet_policy = wallet.policies(KeychainKind::External)?.unwrap();
        let mut path = BTreeMap::new();
        path.insert(wallet_policy.id, vec![1]);

        // Build the transaction
        let mut builder = wallet.build_tx();
        builder
            .add_recipient(to_address.script_pubkey(), amount)
            .policy_path(path, KeychainKind::External);

        // Build the PSBT
        let (psbt, _details) = builder.finish()?;

        let memo: &str = &memo.into();

        // Create spending proposal
        let proposal = SpendingProposal::new(to_address, amount, memo, psbt);
        let extracted_pubkeys = util::extract_public_keys(policy.descriptor.to_string())?;
        let mut tags: Vec<Tag> = extracted_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(policy_id, None, None));
        let content = nips::nip04::encrypt(
            &shared_keys.secret_key()?,
            &shared_keys.public_key(),
            proposal.as_json(),
        )?;
        // Publish proposal with `shared_key` so every owner can delete it
        let event =
            EventBuilder::new(SPENDING_PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;
        let proposal_id = self.client.send_event(event).await?;

        // Send DM msg
        let sender = self.client.keys().public_key();
        let mut msg = String::from("New spending proposal:\n");
        msg.push_str(&format!(
            "- Amount: {} sats\n",
            util::format::big_number(amount)
        ));
        msg.push_str(&format!("- Memo: {memo}"));
        for pubkey in extracted_pubkeys.into_iter() {
            if sender != pubkey {
                self.client.send_direct_msg(pubkey, &msg).await?;
            }
        }

        Ok(proposal_id)
    }

    pub async fn approve(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<EventId> {
        let keys = self.client.keys();

        // Get proposal
        let (proposal, policy_id, shared_keys) =
            self.get_proposal_by_id(proposal_id, timeout).await?;

        // Get policy id
        let (policy, _shared_keys) = self.get_policy_by_id(policy_id, timeout).await?;

        // Create a BDK wallet
        let mut wallet = self.wallet(policy.descriptor.to_string())?;

        // Add the BDK signer
        let private_key = PrivateKey::new(keys.secret_key()?, self.network);
        let signer = SignerWrapper::new(
            private_key,
            SignerContext::Tap {
                is_internal_key: false,
            },
        );
        let internal_signer = SignerWrapper::new(
            private_key,
            SignerContext::Tap {
                is_internal_key: true,
            },
        );

        wallet.add_signer(KeychainKind::External, SignerOrdering(0), Arc::new(signer));
        wallet.add_signer(
            KeychainKind::External,
            SignerOrdering(0),
            Arc::new(internal_signer),
        );

        // Sign the transaction
        let mut psbt = proposal.psbt.clone();
        let _finalized = wallet.sign(&mut psbt, SignOptions::default())?;
        if psbt != proposal.psbt {
            let content = nips::nip04::encrypt(
                &shared_keys.secret_key()?,
                &shared_keys.public_key(),
                psbt.to_string(),
            )?;
            // Publish approved proposal with `shared_key` so after the broadcast
            // of the transaction it can be deleted
            let event = EventBuilder::new(
                APPROVED_PROPOSAL_KIND,
                content,
                &[
                    Tag::Event(proposal_id, None, None),
                    Tag::Event(policy_id, None, None),
                ],
            )
            .to_event(&shared_keys)?;
            let event_id = self.client.send_event(event).await?;
            Ok(event_id)
        } else {
            // TODO: remove this `panic`
            panic!("PSBT not signed")
        }
    }

    pub async fn broadcast(
        &self,
        proposal_id: EventId,
        blockchain: impl Blockchain,
        timeout: Option<Duration>,
    ) -> Result<Txid> {
        // Get PSBTs
        let (mut base_psbt, psbts) = self
            .get_signed_psbts_by_proposal_id(proposal_id, timeout)
            .await?;

        // Combine PSBTs
        for psbt in psbts {
            base_psbt.combine(psbt)?;
        }

        // Finalize and broadcast the transaction
        base_psbt
            .finalize_mut(SECP256K1)
            .expect("Impissible to finalize PSBT"); // TODO: remove this `expect`
        let finalized_tx = base_psbt.extract_tx();
        #[cfg(not(target_arch = "wasm32"))]
        blockchain.broadcast(&finalized_tx)?;
        #[cfg(target_arch = "wasm32")]
        blockchain.broadcast(&finalized_tx).await?;
        let txid = finalized_tx.txid();

        // Delete the proposal
        if let Err(e) = self.delete_proposal_by_id(proposal_id, timeout).await {
            log::error!("Impossibe to delete proposal {proposal_id}: {e}");
        }

        Ok(txid)
    }
}
