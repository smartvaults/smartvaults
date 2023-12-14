// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};

use nostr_sdk::database::NostrDatabaseExt;
use nostr_sdk::nips::nip04;
use nostr_sdk::{ClientMessage, Event, EventBuilder, EventId, Keys, Kind, Profile, Tag};
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::signer::{SharedSigner, Signer};
use smartvaults_protocol::v1::constants::{SHARED_SIGNERS_KIND, SIGNERS_KIND};
use smartvaults_protocol::v1::util::Encryption;
use smartvaults_protocol::v1::util::Serde;
use smartvaults_sdk_sqlite::model::GetSharedSignerRaw;

use super::{Error, SmartVaults};
use crate::types::{GetAllSigners, GetSharedSigner, GetSigner};

impl SmartVaults {
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_signer_by_id(&self, signer_id: EventId) -> Result<Signer, Error> {
        self.storage.signer(&signer_id).await
    }

    pub async fn delete_signer_by_id(&self, signer_id: EventId) -> Result<(), Error> {
        let keys: Keys = self.keys().await;

        let my_shared_signers = self
            .db
            .get_my_shared_signers_by_signer_id(signer_id)
            .await?;
        let mut tags: Vec<Tag> = Vec::new();

        tags.push(Tag::event(signer_id));

        for (shared_signer_id, public_key) in my_shared_signers.into_iter() {
            tags.push(Tag::public_key(public_key));
            tags.push(Tag::event(shared_signer_id));
        }

        let event = EventBuilder::new(Kind::EventDeletion, "", tags).to_event(&keys)?;
        self.client.send_event(event).await?;

        self.storage.delete_signer(&signer_id).await;

        Ok(())
    }

    pub async fn save_signer(&self, signer: Signer) -> Result<EventId, Error> {
        let keys: Keys = self.keys().await;

        if self
            .storage
            .signer_descriptor_exists(signer.descriptor())
            .await
        {
            return Err(Error::SignerDescriptorAlreadyExists);
        }

        // Compose the event
        let content: String = signer.encrypt_with_keys(&keys)?;

        // Compose signer event
        let event = EventBuilder::new(SIGNERS_KIND, content, []).to_event(&keys)?;

        // Publish the event
        let signer_id = self.client.send_event(event).await?;

        // Save signer in db
        self.storage.save_signer(signer_id, signer).await;

        Ok(signer_id)
    }

    pub async fn smartvaults_signer_exists(&self) -> bool {
        self.storage
            .signer_descriptor_exists(self.default_signer.descriptor())
            .await
    }

    pub async fn save_smartvaults_signer(&self) -> Result<EventId, Error> {
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
    pub async fn get_signers(&self) -> Vec<GetSigner> {
        let mut list: Vec<GetSigner> = self
            .storage
            .signers()
            .await
            .into_iter()
            .map(GetSigner::from)
            .collect();
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
            let signer_descriptor: String = signer.descriptor_public_key()?.to_string();
            if descriptor.contains(&signer_descriptor) {
                return Ok(signer);
            }
        }
        Err(Error::SignerNotFound)
    }

    pub async fn share_signer(
        &self,
        signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<EventId, Error> {
        if !self
            .db
            .my_shared_signer_already_shared(signer_id, public_key)
            .await?
        {
            let keys: Keys = self.keys().await;
            let signer: Signer = self.get_signer_by_id(signer_id).await?;
            let shared_signer: SharedSigner = signer.to_shared_signer();
            let content: String =
                nip04::encrypt(&keys.secret_key()?, &public_key, shared_signer.as_json())?;
            let tags = [Tag::event(signer_id), Tag::public_key(public_key)];
            let event: Event =
                EventBuilder::new(SHARED_SIGNERS_KIND, content, tags).to_event(&keys)?;
            let event_id = self.client.send_event(event).await?;
            self.db
                .save_my_shared_signer(signer_id, event_id, public_key)
                .await?;
            Ok(event_id)
        } else {
            Err(Error::SignerAlreadyShared)
        }
    }

    pub async fn share_signer_to_multiple_public_keys(
        &self,
        signer_id: EventId,
        public_keys: Vec<XOnlyPublicKey>,
    ) -> Result<(), Error> {
        if public_keys.is_empty() {
            return Err(Error::NotEnoughPublicKeys);
        }

        let keys: Keys = self.keys().await;
        let signer: Signer = self.get_signer_by_id(signer_id).await?;
        let shared_signer: SharedSigner = signer.to_shared_signer();

        for public_key in public_keys.into_iter() {
            if self
                .db
                .my_shared_signer_already_shared(signer_id, public_key)
                .await?
            {
                tracing::warn!("Signer {signer_id} already shared with {public_key}");
            } else {
                let content: String =
                    nip04::encrypt(&keys.secret_key()?, &public_key, shared_signer.as_json())?;
                let tags = [Tag::event(signer_id), Tag::public_key(public_key)];
                let event: Event =
                    EventBuilder::new(SHARED_SIGNERS_KIND, content, tags).to_event(&keys)?;
                let event_id: EventId = event.id;

                // TODO: use send_batch_event method from nostr-sdk
                self.client
                    .pool()
                    .send_msg(ClientMessage::new_event(event), None)
                    .await?;

                self.db
                    .save_my_shared_signer(signer_id, event_id, public_key)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn revoke_all_shared_signers(&self) -> Result<(), Error> {
        let keys: Keys = self.keys().await;
        for (shared_signer_id, public_key) in self.db.get_my_shared_signers().await?.into_iter() {
            let tags = [Tag::public_key(public_key), Tag::event(shared_signer_id)];
            let event = EventBuilder::new(Kind::EventDeletion, "", tags).to_event(&keys)?;
            self.client.send_event(event).await?;
            self.db.delete_shared_signer(shared_signer_id).await?;
        }
        Ok(())
    }

    pub async fn revoke_shared_signer(&self, shared_signer_id: EventId) -> Result<(), Error> {
        let keys: Keys = self.keys().await;
        let public_key = self
            .db
            .get_public_key_for_my_shared_signer(shared_signer_id)
            .await?;
        let tags = [Tag::public_key(public_key), Tag::event(shared_signer_id)];
        let event = EventBuilder::new(Kind::EventDeletion, "", tags).to_event(&keys)?;
        self.client.send_event(event).await?;
        self.db.delete_shared_signer(shared_signer_id).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn my_shared_signer_already_shared(
        &self,
        signer_id: EventId,
        public_key: XOnlyPublicKey,
    ) -> Result<bool, Error> {
        Ok(self
            .db
            .my_shared_signer_already_shared(signer_id, public_key)
            .await?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_my_shared_signers_by_signer_id(
        &self,
        signer_id: EventId,
    ) -> Result<BTreeMap<EventId, Profile>, Error> {
        let mut map = BTreeMap::new();
        let ssbs = self
            .db
            .get_my_shared_signers_by_signer_id(signer_id)
            .await?;
        for (key, pk) in ssbs.into_iter() {
            let profile: Profile = self.client.database().profile(pk).await?;
            map.insert(key, profile);
        }
        Ok(map)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_shared_signers(&self) -> Result<Vec<GetSharedSigner>, Error> {
        let mut list = Vec::new();
        let ss = self.db.get_shared_signers().await?;
        for GetSharedSignerRaw {
            shared_signer_id,
            owner_public_key,
            shared_signer,
        } in ss.into_iter()
        {
            let profile: Profile = self.client.database().profile(owner_public_key).await?;
            list.push(GetSharedSigner {
                shared_signer_id,
                owner: profile,
                shared_signer,
            });
        }
        Ok(list)
    }

    pub async fn get_shared_signers_public_keys(
        &self,
        include_contacts: bool,
    ) -> Result<Vec<XOnlyPublicKey>, Error> {
        let public_keys: HashSet<XOnlyPublicKey> = self.db.get_shared_signers_public_keys().await?;
        if include_contacts {
            Ok(public_keys.into_iter().collect())
        } else {
            let keys = self.client.keys().await;
            let contacts: Vec<XOnlyPublicKey> = self
                .client
                .database()
                .contacts_public_keys(keys.public_key())
                .await?;
            let contacts: HashSet<XOnlyPublicKey> = contacts.into_iter().collect();
            Ok(public_keys.difference(&contacts).copied().collect())
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_shared_signers_by_public_key(
        &self,
        public_key: XOnlyPublicKey,
    ) -> Result<Vec<GetSharedSigner>, Error> {
        let profile: Profile = self.client.database().profile(public_key).await?;
        Ok(self
            .db
            .get_shared_signers_by_public_key(public_key)
            .await?
            .into_iter()
            .map(
                |GetSharedSignerRaw {
                     shared_signer_id,
                     shared_signer,
                     ..
                 }| GetSharedSigner {
                    shared_signer_id,
                    owner: profile.clone(),
                    shared_signer,
                },
            )
            .collect())
    }
}
