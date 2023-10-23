// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr::nips::nip04;
use nostr::{Event, EventBuilder, EventId, Keys, Tag};
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::{Policy, Proposal};
use thiserror::Error;

use super::constants::{
    KEY_AGENT_SIGNER_OFFERING_KIND, LABELS_KIND, POLICY_KIND, PROPOSAL_KIND, SHARED_KEY_KIND,
};
use super::key_agent::signer::SignerOffering;
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
        receiver: &XOnlyPublicKey,
        policy_id: EventId,
    ) -> Result<Event, Error> {
        let encrypted_shared_key = nip04::encrypt(
            &keys.secret_key()?,
            receiver,
            shared_key.secret_key()?.display_secret().to_string(),
        )?;
        let event: Event = EventBuilder::new(
            SHARED_KEY_KIND,
            encrypted_shared_key,
            &[
                Tag::Event(policy_id, None, None),
                Tag::PubKey(*receiver, None),
            ],
        )
        .to_event(keys)?;
        Ok(event)
    }

    fn policy(
        shared_key: &Keys,
        policy: &Policy,
        nostr_pubkeys: &[XOnlyPublicKey],
    ) -> Result<Event, Error> {
        let content: String = policy.encrypt_with_keys(shared_key)?;
        let tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        Ok(EventBuilder::new(POLICY_KIND, content, &tags).to_event(shared_key)?)
    }

    fn proposal(
        shared_key: &Keys,
        policy_id: EventId,
        proposal: &Proposal,
        nostr_pubkeys: &[XOnlyPublicKey],
    ) -> Result<Event, Error> {
        let mut tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(policy_id, None, None));
        let content: String = proposal.encrypt_with_keys(shared_key)?;
        Ok(EventBuilder::new(PROPOSAL_KIND, content, &tags).to_event(shared_key)?)
    }

    fn label(
        shared_key: &Keys,
        policy_id: EventId,
        label: &Label,
        nostr_pubkeys: &[XOnlyPublicKey],
    ) -> Result<Event, Error> {
        let identifier: String = label.generate_identifier(shared_key)?;
        let content: String = label.encrypt_with_keys(shared_key)?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Identifier(identifier));
        tags.push(Tag::Event(policy_id, None, None));
        Ok(EventBuilder::new(LABELS_KIND, content, &tags).to_event(shared_key)?)
    }

    fn signer_offering(keys: &Keys, id: String, offering: SignerOffering) -> Result<Event, Error> {
        let content: String = offering.as_json();
        Ok(EventBuilder::new(
            KEY_AGENT_SIGNER_OFFERING_KIND,
            content,
            &[Tag::Identifier(id)],
        )
        .to_event(keys)?)
    }
}

impl SmartVaultsEventBuilder for EventBuilder {}
