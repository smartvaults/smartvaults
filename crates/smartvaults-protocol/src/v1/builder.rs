// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashMap;

use nostr::nips::nip04;
use nostr::{Event, EventBuilder, EventId, Keys, PublicKey, Tag};
use smartvaults_core::bitcoin::Network;
use smartvaults_core::{Policy, Proposal, Signer};
use thiserror::Error;

use super::constants::{
    KEY_AGENT_SIGNALING, KEY_AGENT_SIGNER_OFFERING_KIND, KEY_AGENT_VERIFIED, LABELS_KIND,
    POLICY_KIND, PROPOSAL_KIND, SHARED_KEY_KIND,
};
use super::key_agent::signer::SignerOffering;
use super::key_agent::verified::VerifiedKeyAgentData;
use super::util::{Encryption, EncryptionError};
use super::{Label, Serde};

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Keys(#[from] nostr::key::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr::event::builder::Error),
    #[error(transparent)]
    NIP04(#[from] nostr::nips::nip04::Error),
    #[error(transparent)]
    Encryption(#[from] EncryptionError),
    #[error(transparent)]
    Label(#[from] super::label::Error),
}

pub trait SmartVaultsEventBuilder {
    fn shared_key(
        keys: &Keys,
        shared_key: &Keys,
        receiver: &PublicKey,
        policy_id: EventId,
    ) -> Result<Event, Error> {
        let encrypted_shared_key = nip04::encrypt(
            keys.secret_key()?,
            receiver,
            shared_key.secret_key()?.display_secret().to_string(),
        )?;
        let event: Event = EventBuilder::new(
            SHARED_KEY_KIND,
            encrypted_shared_key,
            [Tag::event(policy_id), Tag::public_key(*receiver)],
        )
        .to_event(keys)?;
        Ok(event)
    }

    fn policy(
        shared_key: &Keys,
        policy: &Policy,
        nostr_pubkeys: &[PublicKey],
    ) -> Result<Event, Error> {
        let content: String = policy.encrypt_with_keys(shared_key)?;
        let tags = nostr_pubkeys.iter().copied().map(Tag::public_key);
        Ok(EventBuilder::new(POLICY_KIND, content, tags).to_event(shared_key)?)
    }

    fn proposal(
        shared_key: &Keys,
        policy_id: EventId,
        proposal: &Proposal,
        nostr_pubkeys: &[PublicKey],
    ) -> Result<Event, Error> {
        let mut tags: Vec<Tag> = nostr_pubkeys.iter().copied().map(Tag::public_key).collect();
        tags.push(Tag::event(policy_id));
        let content: String = proposal.encrypt_with_keys(shared_key)?;
        Ok(EventBuilder::new(PROPOSAL_KIND, content, tags).to_event(shared_key)?)
    }

    fn label(
        shared_key: &Keys,
        policy_id: EventId,
        label: &Label,
        nostr_pubkeys: &[PublicKey],
    ) -> Result<Event, Error> {
        let identifier: String = label.generate_identifier(shared_key)?;
        let content: String = label.encrypt_with_keys(shared_key)?;
        let mut tags: Vec<Tag> = nostr_pubkeys.iter().copied().map(Tag::public_key).collect();
        tags.push(Tag::Identifier(identifier));
        tags.push(Tag::event(policy_id));
        Ok(EventBuilder::new(LABELS_KIND, content, tags).to_event(shared_key)?)
    }

    fn key_agent_signaling(keys: &Keys, network: Network) -> Result<Event, Error> {
        let identifier: String = network.magic().to_string();
        Ok(
            EventBuilder::new(KEY_AGENT_SIGNALING, "", [Tag::Identifier(identifier)])
                .to_event(keys)?,
        )
    }

    fn signer_offering(
        keys: &Keys,
        signer: &Signer,
        offering: &SignerOffering,
        network: Network,
    ) -> Result<Event, Error> {
        let content: String = offering.as_json();
        Ok(EventBuilder::new(
            KEY_AGENT_SIGNER_OFFERING_KIND,
            content,
            [Tag::Identifier(signer.generate_identifier(network))],
        )
        .to_event(keys)?)
    }

    fn key_agents_verified(
        keys: &Keys,
        public_keys: HashMap<PublicKey, VerifiedKeyAgentData>,
        network: Network,
    ) -> Result<Event, Error> {
        let identifier: String = network.magic().to_string();
        let content: String = serde_json::json!(public_keys).to_string();
        let mut tags: Vec<Tag> = Vec::with_capacity(1 + public_keys.len());
        tags.push(Tag::Identifier(identifier));
        Ok(EventBuilder::new(KEY_AGENT_VERIFIED, content, tags).to_event(keys)?)
    }
}

impl SmartVaultsEventBuilder for EventBuilder {}
