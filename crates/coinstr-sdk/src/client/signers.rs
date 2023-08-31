// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};

use coinstr_core::miniscript::Descriptor;
use coinstr_core::secp256k1::XOnlyPublicKey;
use coinstr_core::signer::{coinstr_signer, SharedSigner, Signer};
use coinstr_protocol::v1::constants::{SHARED_SIGNERS_KIND, SIGNERS_KIND};
use coinstr_protocol::v1::util::Encryption;
use coinstr_protocol::v1::util::Serde;
use nostr_sdk::nips::nip04;
use nostr_sdk::{ClientMessage, Event, EventBuilder, EventId, Keys, Kind, Tag};

use super::{Coinstr, Error};
use crate::db::model::{GetAllSigners, GetSharedSigner, GetSigner};

impl Coinstr {
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_signer_by_id(&self, signer_id: EventId) -> Result<Signer, Error> {
        Ok(self.db.get_signer_by_id(signer_id).await?)
    }

    pub async fn delete_signer_by_id(&self, signer_id: EventId) -> Result<(), Error> {
        let keys = self.client.keys();

        let my_shared_signers = self
            .db
            .get_my_shared_signers_by_signer_id(signer_id)
            .await?;
        let mut tags: Vec<Tag> = Vec::new();

        tags.push(Tag::Event(signer_id, None, None));

        for (shared_signer_id, public_key) in my_shared_signers.into_iter() {
            tags.push(Tag::PubKey(public_key, None));
            tags.push(Tag::Event(shared_signer_id, None, None));
        }

        let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&keys)?;
        self.send_event(event).await?;

        self.db.delete_signer(signer_id).await?;

        Ok(())
    }

    pub async fn save_signer(&self, signer: Signer) -> Result<EventId, Error> {
        let keys = self.client.keys();

        if self
            .db
            .signer_descriptor_exists(signer.descriptor())
            .await?
        {
            return Err(Error::SignerDescriptorAlreadyExists);
        }

        // Compose the event
        let content: String = signer.encrypt_with_keys(&keys)?;

        // Compose signer event
        let event = EventBuilder::new(SIGNERS_KIND, content, &[]).to_event(&keys)?;

        // Publish the event
        let signer_id = self.send_event(event).await?;

        // Save signer in db
        self.db.save_signer(signer_id, signer).await?;

        Ok(signer_id)
    }

    pub async fn coinstr_signer_exists(&self) -> Result<bool, Error> {
        let signer = coinstr_signer(self.keechain.keychain.seed(), self.network)?;
        Ok(self
            .db
            .signer_descriptor_exists(signer.descriptor())
            .await?)
    }

    pub async fn save_coinstr_signer(&self) -> Result<EventId, Error> {
        let signer = coinstr_signer(self.keechain.keychain.seed(), self.network)?;
        self.save_signer(signer).await
    }

    /// Get all own signers and contacts shared signers
    pub async fn get_all_signers(&self) -> Result<GetAllSigners, Error> {
        Ok(GetAllSigners {
            my: self.get_signers().await?,
            contacts: self.get_shared_signers().await?,
        })
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_signers(&self) -> Result<Vec<GetSigner>, Error> {
        Ok(self.db.get_signers().await?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn search_signer_by_descriptor(
        &self,
        descriptor: Descriptor<String>,
    ) -> Result<Signer, Error> {
        let descriptor: String = descriptor.to_string();
        for GetSigner { signer, .. } in self.db.get_signers().await?.into_iter() {
            let signer_descriptor = signer.descriptor_public_key()?.to_string();
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
            let keys: Keys = self.client.keys();
            let signer: Signer = self.get_signer_by_id(signer_id).await?;
            let shared_signer: SharedSigner = signer.to_shared_signer();
            let content: String =
                nip04::encrypt(&keys.secret_key()?, &public_key, shared_signer.as_json())?;
            let tags = &[
                Tag::Event(signer_id, None, None),
                Tag::PubKey(public_key, None),
            ];
            let event: Event =
                EventBuilder::new(SHARED_SIGNERS_KIND, content, tags).to_event(&keys)?;
            let event_id = self.send_event(event).await?;
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

        let keys: Keys = self.client.keys();
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
                let tags = &[
                    Tag::Event(signer_id, None, None),
                    Tag::PubKey(public_key, None),
                ];
                let event: Event =
                    EventBuilder::new(SHARED_SIGNERS_KIND, content, tags).to_event(&keys)?;
                let event_id: EventId = event.id;

                // TODO: use send_batch_event method from nostr-sdk
                self.db.save_event(event.clone()).await?;
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
        let keys = self.client.keys();
        for (shared_signer_id, public_key) in self.db.get_my_shared_signers().await?.into_iter() {
            let tags = &[
                Tag::PubKey(public_key, None),
                Tag::Event(shared_signer_id, None, None),
            ];
            let event = EventBuilder::new(Kind::EventDeletion, "", tags).to_event(&keys)?;
            self.send_event(event).await?;
            self.db.delete_shared_signer(shared_signer_id).await?;
        }
        Ok(())
    }

    pub async fn revoke_shared_signer(&self, shared_signer_id: EventId) -> Result<(), Error> {
        let keys = self.client.keys();
        let public_key = self
            .db
            .get_public_key_for_my_shared_signer(shared_signer_id)
            .await?;
        let tags = &[
            Tag::PubKey(public_key, None),
            Tag::Event(shared_signer_id, None, None),
        ];
        let event = EventBuilder::new(Kind::EventDeletion, "", tags).to_event(&keys)?;
        self.send_event(event).await?;
        self.db.delete_shared_signer(shared_signer_id).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_my_shared_signers_by_signer_id(
        &self,
        signer_id: EventId,
    ) -> Result<BTreeMap<EventId, XOnlyPublicKey>, Error> {
        Ok(self
            .db
            .get_my_shared_signers_by_signer_id(signer_id)
            .await?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_shared_signers(&self) -> Result<Vec<GetSharedSigner>, Error> {
        Ok(self.db.get_shared_signers().await?)
    }

    pub async fn get_shared_signers_public_keys(
        &self,
        include_contacts: bool,
    ) -> Result<Vec<XOnlyPublicKey>, Error> {
        let public_keys: HashSet<XOnlyPublicKey> = self.db.get_shared_signers_public_keys().await?;
        if include_contacts {
            Ok(public_keys.into_iter().collect())
        } else {
            let contacts: HashSet<XOnlyPublicKey> = self.db.get_contacts_public_keys().await?;
            Ok(public_keys.difference(&contacts).copied().collect())
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_shared_signers_by_public_key(
        &self,
        public_key: XOnlyPublicKey,
    ) -> Result<Vec<GetSharedSigner>, Error> {
        Ok(self.db.get_shared_signers_by_public_key(public_key).await?)
    }
}
