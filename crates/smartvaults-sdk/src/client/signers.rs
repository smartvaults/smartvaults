// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashSet;

use nostr_sdk::prelude::*;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_protocol::v2::constants::{SHARED_SIGNER_KIND_V2, SIGNER_KIND_V2};
use smartvaults_protocol::v2::{
    self, NostrPublicIdentifier, SharedSigner, Signer, SignerIdentifier,
};

use super::{Error, SmartVaults};
use crate::types::{GetAllSigners, GetSharedSigner};

impl SmartVaults {
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_signer_by_id(&self, signer_id: &SignerIdentifier) -> Result<Signer, Error> {
        Ok(self.storage.signer(&signer_id).await?)
    }

    pub async fn delete_signer_by_id(&self, signer_id: &SignerIdentifier) -> Result<(), Error> {
        let signer: Signer = self.storage.signer(signer_id).await?;

        let nostr_public_identifier: NostrPublicIdentifier = signer.nostr_public_identifier();

        let filter: Filter = Filter::new()
            .kind(SIGNER_KIND_V2)
            .author(self.keys.public_key())
            .identifier(nostr_public_identifier.to_string())
            .limit(1);
        let res: Vec<Event> = self
            .client
            .database()
            .query(vec![filter], Order::Desc)
            .await?;
        let signer_event: &Event = res.first().ok_or(Error::NotFound)?;

        let event = EventBuilder::new(Kind::EventDeletion, "", [Tag::event(signer_event.id)]);
        self.client.send_event_builder(event).await?;

        self.storage.delete_signer(&signer_id).await;

        Ok(())
    }

    pub async fn save_signer(&self, signer: Signer) -> Result<SignerIdentifier, Error> {
        let keys: &Keys = self.keys();

        // Compose and publish event
        let event: Event = v2::signer::build_event(keys, &signer)?;
        self.client.send_event(event).await?;

        // Index signer
        let id: SignerIdentifier = signer.id();
        self.storage.save_signer(id, signer).await;

        Ok(id)
    }

    pub async fn smartvaults_signer_exists(&self) -> bool {
        self.storage.signer_exists(&self.default_signer.id()).await
    }

    pub async fn save_smartvaults_signer(&self) -> Result<SignerIdentifier, Error> {
        self.save_signer(self.default_signer.clone()).await
    }

    /// Get all own signers and contacts shared signers
    pub async fn get_all_signers(&self) -> Result<GetAllSigners, Error> {
        Ok(GetAllSigners {
            my: self.get_signers().await,
            contacts: self.get_shared_signers().await?,
        })
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_signers(&self) -> Vec<Signer> {
        let mut list: Vec<Signer> = self.storage.signers().await.into_values().collect();
        list.sort();
        list
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn search_signer_by_descriptor(
        &self,
        descriptor: Descriptor<String>,
    ) -> Result<Signer, Error> {
        let descriptor: String = descriptor.to_string();
        for signer in self.storage.signers().await.into_values() {
            for desc in signer.descriptors().values() {
                let signer_descriptor: String = desc.to_string();
                if descriptor.contains(&signer_descriptor) {
                    return Ok(signer);
                }
            }
        }
        Err(Error::SignerNotFound)
    }

    pub async fn share_signer(
        &self,
        signer_id: &SignerIdentifier,
        receiver: PublicKey,
    ) -> Result<EventId, Error> {
        let keys: &Keys = self.keys();
        let signer: Signer = self.get_signer_by_id(signer_id).await?;
        let _shared_signer: SharedSigner = signer.to_shared(keys.public_key(), receiver);

        todo!();

        // self.storage
        // .save_my_shared_signer(signer_id, event_id, public_key)
        // .await;
        // Ok(event_id)
    }

    pub async fn delete_shared_signer(
        &self,
        shared_signer_id: &NostrPublicIdentifier,
    ) -> Result<(), Error> {
        let filter: Filter = Filter::new()
            .kind(SHARED_SIGNER_KIND_V2)
            .author(self.keys.public_key())
            .identifier(shared_signer_id.to_string())
            .limit(1);
        let res: Vec<Event> = self
            .client
            .database()
            .query(vec![filter], Order::Desc)
            .await?;
        let shared_signer_event: &Event = res.first().ok_or(Error::NotFound)?;

        let event = EventBuilder::new(
            Kind::EventDeletion,
            "",
            [Tag::event(shared_signer_event.id)],
        );
        self.client.send_event_builder(event).await?;

        self.storage.delete_shared_signer(shared_signer_id).await;

        Ok(())
    }

    pub async fn share_signer_to_multiple_public_keys(
        &self,
        signer_id: &SignerIdentifier,
        receivers: Vec<PublicKey>,
    ) -> Result<(), Error> {
        if receivers.is_empty() {
            return Err(Error::NotEnoughPublicKeys);
        }

        let keys: &Keys = self.keys();
        let signer: Signer = self.get_signer_by_id(signer_id).await?;

        for receiver in receivers.into_iter() {
            let _shared_signer: SharedSigner = signer.as_shared(keys.public_key(), receiver);

            todo!();

            // TODO: use send_batch_event method from nostr-sdk
            // self.client
            // .pool()
            // .send_msg(
            //      ClientMessage::event(event),
            //      RelaySendOptions::new().skip_send_confirmation(true),
            //  )
            // .await?;
            //
            // self.storage
            // .save_my_shared_signer(signer_id, event_id, public_key)
            // .await;
        }

        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_shared_signers(&self) -> Result<Vec<GetSharedSigner>, Error> {
        let mut list = Vec::new();
        for (shared_signer_id, shared_signer) in self.storage.shared_signers().await.into_iter() {
            let profile: Profile = self
                .client
                .database()
                .profile(*shared_signer.owner())
                .await?;
            list.push(GetSharedSigner {
                shared_signer_id,
                owner: profile,
                shared_signer,
            });
        }
        list.sort();
        Ok(list)
    }

    pub async fn get_shared_signers_public_keys(
        &self,
        include_contacts: bool,
    ) -> Result<Vec<PublicKey>, Error> {
        let public_keys: HashSet<PublicKey> = self.storage.get_shared_signers_public_keys().await;
        if include_contacts {
            Ok(public_keys.into_iter().collect())
        } else {
            let keys = self.keys();
            let contacts: Vec<PublicKey> = self
                .client
                .database()
                .contacts_public_keys(keys.public_key())
                .await?;
            let contacts: HashSet<PublicKey> = contacts.into_iter().collect();
            Ok(public_keys.difference(&contacts).copied().collect())
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_shared_signers_by_public_key(
        &self,
        public_key: PublicKey,
    ) -> Result<Vec<GetSharedSigner>, Error> {
        let profile: Profile = self.client.database().profile(public_key).await?;
        Ok(self
            .storage
            .get_shared_signers_by_public_key(public_key)
            .await
            .into_iter()
            .map(|(shared_signer_id, shared_signer)| GetSharedSigner {
                shared_signer_id,
                owner: profile.clone(),
                shared_signer,
            })
            .collect())
    }
}
