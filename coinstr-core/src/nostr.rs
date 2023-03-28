use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::Txid;
use bdk::blockchain::Blockchain;
use bdk::miniscript::psbt::PsbtExt;
use nostr_sdk::blocking::Client;
use nostr_sdk::secp256k1::SecretKey;
use nostr_sdk::{nips, EventBuilder, EventId, Filter, Keys, Result, Tag, SECP256K1};

use crate::constants::{
    APPROVED_PROPOSAL_KIND, POLICY_KIND, SHARED_KEY_KIND, SPENDING_PROPOSAL_KIND,
};
use crate::policy::Policy;
use crate::proposal::SpendingProposal;
use crate::util;

pub trait CoinstrNostr {
    fn get_shared_keys(&self, timeout: Option<Duration>) -> Result<HashMap<EventId, Keys>>;

    fn get_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Policy, Keys)>;

    fn get_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(SpendingProposal, EventId, Keys)>;

    fn get_signed_psbts_by_proposal_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(PartiallySignedTransaction, Vec<PartiallySignedTransaction>)>;

    fn delete_policy_by_id(&self, policy_id: EventId, timeout: Option<Duration>) -> Result<()>;

    fn delete_proposal_by_id(&self, proposal_id: EventId, timeout: Option<Duration>) -> Result<()>;

    fn get_policies(&self, timeout: Option<Duration>) -> Result<Vec<(EventId, Policy)>>;

    fn get_proposals(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<(EventId, SpendingProposal, EventId)>>;

    fn save_policy<S>(&self, name: S, description: S, descriptor: S) -> Result<EventId>
    where
        S: Into<String>;

    /*     fn spend<S>(&self, policy_id: EventId, to_address: Address, amount: u64, memo: S, timeout: Option<Duration>) -> Result<EventId>
    where
        S: Into<String>; */

    fn broadcast(
        &self,
        proposal_id: EventId,
        blockchain: impl Blockchain,
        timeout: Option<Duration>,
    ) -> Result<Txid>;
}

impl CoinstrNostr for Client {
    fn get_shared_keys(&self, timeout: Option<Duration>) -> Result<HashMap<EventId, Keys>> {
        let keys = self.keys();

        let filter = Filter::new()
            .pubkey(keys.public_key())
            .kind(SHARED_KEY_KIND);
        let global_shared_key_events = self.get_events_of(vec![filter], timeout)?;

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

    fn get_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Policy, Keys)> {
        let keys = self.keys();

        // Get policy event
        let filter = Filter::new().id(policy_id).kind(POLICY_KIND);
        let events = self.get_events_of(vec![filter], timeout)?;
        let policy_event = events.first().expect("Policy not found");

        // Get global shared key
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.get_events_of(vec![filter], timeout)?;
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

    fn get_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(SpendingProposal, EventId, Keys)> {
        let keys = self.keys();

        // Get proposal event
        let filter = Filter::new().id(proposal_id).kind(SPENDING_PROPOSAL_KIND);
        let events = self.get_events_of(vec![filter], timeout)?;
        let proposal_event = events.first().expect("Spending proposal not found");
        let policy_id = util::extract_first_event_id(proposal_event).expect("Policy id not found");

        // Get global shared key
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.get_events_of(vec![filter], timeout)?;
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

    fn get_signed_psbts_by_proposal_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(PartiallySignedTransaction, Vec<PartiallySignedTransaction>)> {
        // Get approved proposals
        let filter = Filter::new()
            .event(proposal_id)
            .kind(APPROVED_PROPOSAL_KIND);
        let proposals_events = self.get_events_of(vec![filter], timeout)?;
        let first_event = proposals_events
            .first()
            .expect("Approved proposals not found");
        let proposal_id = util::extract_first_event_id(first_event).expect("Proposal id not found");

        // Get global shared key
        let (proposal, _, shared_keys) = self.get_proposal_by_id(proposal_id, timeout)?;

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

    fn delete_policy_by_id(&self, policy_id: EventId, timeout: Option<Duration>) -> Result<()> {
        let keys = self.keys();

        // Get global shared key
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.get_events_of(vec![filter], timeout)?;
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
        let events = self.get_events_of(vec![filter], timeout)?;

        let mut ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
        ids.push(policy_id);

        let event = EventBuilder::delete::<String>(ids, None).to_event(&shared_keys)?;
        self.send_event(event)?;

        Ok(())
    }

    fn delete_proposal_by_id(&self, proposal_id: EventId, timeout: Option<Duration>) -> Result<()> {
        let keys = self.keys();

        // Get the proposal
        let filter = Filter::new().id(proposal_id);
        let events = self.get_events_of(vec![filter], timeout)?;
        let proposal_event = events.first().expect("Spending proposal not found");
        let policy_id = util::extract_first_event_id(proposal_event).expect("Policy id not found");

        // Get global shared key
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.get_events_of(vec![filter], timeout)?;
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
        let events = self.get_events_of(vec![filter], timeout)?;

        let mut ids: Vec<EventId> = events.iter().map(|e| e.id).collect();
        ids.push(proposal_id);

        let event = EventBuilder::delete::<String>(ids, None).to_event(&shared_keys)?;
        self.send_event(event)?;

        Ok(())
    }

    fn get_policies(&self, timeout: Option<Duration>) -> Result<Vec<(EventId, Policy)>> {
        let keys = self.keys();

        // Get policies
        let filter = Filter::new().pubkey(keys.public_key()).kind(POLICY_KIND);
        let policies_events = self.get_events_of(vec![filter], timeout)?;

        // Get shared keys
        let shared_keys: HashMap<EventId, Keys> = self.get_shared_keys(timeout)?;

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

    fn get_proposals(
        &self,
        timeout: Option<Duration>,
    ) -> Result<Vec<(EventId, SpendingProposal, EventId)>> {
        let keys = self.keys();

        // Get proposals
        let filter = Filter::new()
            .pubkey(keys.public_key())
            .kind(SPENDING_PROPOSAL_KIND);
        let proposals_events = self.get_events_of(vec![filter], timeout)?;

        // Get shared keys
        let shared_keys: HashMap<EventId, Keys> = self.get_shared_keys(timeout)?;

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

    fn save_policy<S>(&self, name: S, description: S, descriptor: S) -> Result<EventId>
    where
        S: Into<String>,
    {
        let keys = self.keys();

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
        let policy_id = self.send_event(policy_event)?;

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
            let event_id = self.send_event(event)?;
            log::info!("Published shared key for {pubkey} at event {event_id}");
        }

        Ok(policy_id)
    }

    fn broadcast(
        &self,
        proposal_id: EventId,
        blockchain: impl Blockchain,
        timeout: Option<Duration>,
    ) -> Result<Txid> {
        // Get PSBTs
        let (mut base_psbt, psbts) = self.get_signed_psbts_by_proposal_id(proposal_id, timeout)?;

        // Combine PSBTs
        for psbt in psbts {
            base_psbt.combine(psbt)?;
        }

        // Finalize and broadcast the transaction
        base_psbt
            .finalize_mut(SECP256K1)
            .expect("Impissible to finalize PSBT"); // TODO: remove this `expect`
        let finalized_tx = base_psbt.extract_tx();
        blockchain.broadcast(&finalized_tx)?;
        let txid = finalized_tx.txid();

        // Delete the proposal
        if let Err(e) = self.delete_proposal_by_id(proposal_id, timeout) {
            log::error!("Impossibe to delete proposal {proposal_id}: {e}");
        }

        Ok(txid)
    }
}
