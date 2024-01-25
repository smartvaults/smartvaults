// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeSet;
use std::ops::Deref;

use nostr_sdk::database::Order;
use nostr_sdk::{
    Event, EventBuilder, Filter, Keys, Kind, NostrDatabaseExt, Profile, RelaySendOptions, Tag,
};
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::{Policy, PolicyTemplate};
use smartvaults_protocol::v2::constants::VAULT_KIND_V2;
use smartvaults_protocol::v2::{
    self, NostrPublicIdentifier, Vault, VaultIdentifier, VaultInvite, VaultMetadata,
};

use super::{Error, SmartVaults};
use crate::storage::InternalVault;
use crate::types::GetVault;

impl SmartVaults {
    /// Get own vaults
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn vaults(&self) -> Result<Vec<GetVault>, Error> {
        let items = self.storage.vaults().await;
        let mut vaults: Vec<GetVault> = Vec::with_capacity(items.len());

        for (id, InternalVault { vault, metadata }) in items.into_iter() {
            vaults.push(GetVault {
                vault,
                metadata,
                balance: self.manager.get_balance(&id).await?,
                last_sync: self.manager.last_sync(&id).await?,
            });
        }

        vaults.sort();

        Ok(vaults)
    }

    /// Get vault by [VaultIdentifier]
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_vault_by_id(&self, vault_id: &VaultIdentifier) -> Result<GetVault, Error> {
        let InternalVault { vault, metadata } = self.storage.vault(vault_id).await?;
        Ok(GetVault {
            vault,
            metadata,
            balance: self.manager.get_balance(vault_id).await?,
            last_sync: self.manager.last_sync(vault_id).await?,
        })
    }

    async fn internal_save_vault(
        &self,
        vault: Vault,
        metadata: Option<VaultMetadata>,
    ) -> Result<VaultIdentifier, Error> {
        let vault_id: VaultIdentifier = vault.id();

        // TODO: check if vault already exists

        // Compose and publish events
        let keys = self.keys();
        let mut events: Vec<Event> = Vec::with_capacity(1 + usize::from(metadata.is_some()));
        events.push(v2::vault::build_event(keys, &vault)?);
        if let Some(metadata) = &metadata {
            events.push(v2::vault::metadata::build_event(&vault, metadata)?);
        }
        self.client
            .batch_event(events, RelaySendOptions::new())
            .await?;

        // Load policy
        let policy: Policy = vault.policy();
        self.manager.load_policy(vault_id, policy).await?;

        // Index event
        self.storage.save_vault(vault_id, vault, metadata).await;

        // Mark for re-subscription
        self.set_resubscribe_vaults(true);

        Ok(vault_id)
    }

    pub async fn save_vault<S, D>(
        &self,
        name: S,
        description: S,
        descriptor: D,
    ) -> Result<VaultIdentifier, Error>
    where
        S: Into<String>,
        D: AsRef<str>,
    {
        // Generate a shared key
        let shared_key = Keys::generate();
        let vault = Vault::new(descriptor, self.network, shared_key.secret_key()?)?;
        let vault_id: VaultIdentifier = vault.id();

        // Add metadata
        let mut metadata = VaultMetadata::new(vault_id);
        metadata.change_name(name);
        metadata.change_description(description);

        // Save vault
        self.internal_save_vault(vault, Some(metadata)).await
    }

    pub async fn save_vault_from_template<S>(
        &self,
        name: S,
        description: S,
        template: PolicyTemplate,
    ) -> Result<VaultIdentifier, Error>
    where
        S: Into<String>,
    {
        let shared_key = Keys::generate();
        let vault: Vault = Vault::from_template(template, self.network, shared_key.secret_key()?)?;
        let descriptor: String = vault.as_descriptor().to_string();
        self.save_vault(name, description, descriptor).await
    }

    /// Edit [Vault] metadata
    ///
    /// Args set to `None` aren't updated.
    pub async fn edit_vault_metadata(
        &self,
        vault_id: &VaultIdentifier,
        name: Option<String>,
        description: Option<String>,
    ) -> Result<(), Error> {
        let InternalVault {
            vault,
            mut metadata,
        } = self.storage.vault(vault_id).await?;

        if let Some(name) = name {
            metadata.name = name;
        }

        if let Some(description) = description {
            metadata.description = description;
        }

        let event: Event = v2::vault::metadata::build_event(&vault, &metadata)?;
        self.client.send_event(event).await?;

        self.storage.edit_vault_metadata(vault_id, metadata).await;

        Ok(())
    }

    /// Invite an user to a [Vault]
    pub async fn invite_to_vault<S>(
        &self,
        vault_id: &VaultIdentifier,
        receiver: XOnlyPublicKey,
        message: S,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        // Get vailt
        let InternalVault { vault, .. } = self.storage.vault(vault_id).await?;

        // Compose invite
        let sender: XOnlyPublicKey = self.keys.public_key();
        let invite: VaultInvite = VaultInvite::new(vault, Some(sender), message);

        // Compose and publish event
        let event: Event = v2::vault::invite::build_event(invite, receiver)?;
        self.client.send_event(event).await?;

        Ok(())
    }

    /// Get vault invites
    pub async fn vault_invites(&self) -> Result<Vec<VaultInvite>, Error> {
        let invites = self.storage.vault_invites().await;
        let mut invites: Vec<VaultInvite> = invites.into_values().collect();
        invites.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        Ok(invites)
    }

    /// Accept a vault invite
    pub async fn accept_vault_invite(&self, invite: VaultInvite) -> Result<(), Error> {
        self.internal_save_vault(invite.vault, None).await?;
        // TODO: delete invite
        Ok(())
    }

    /// Get members of [Vault]
    pub async fn get_members_of_vault(
        &self,
        vault_id: &VaultIdentifier,
    ) -> Result<BTreeSet<Profile>, Error> {
        // Get vault and shared signers
        let InternalVault { vault, .. } = self.storage.vault(vault_id).await?;
        let shared_signers = self.storage.shared_signers().await;

        // Search used signers using the shared signers
        let result = vault.search_used_signers(shared_signers.values().map(|s| s.deref().clone()));

        // Compose output
        let mut users: BTreeSet<Profile> = BTreeSet::new();
        for shared_signer in shared_signers
            .into_values()
            .filter(|s| result.contains(s.deref()))
        {
            let profile: Profile = self
                .client
                .database()
                .profile(*shared_signer.owner())
                .await?;
            users.insert(profile);
        }

        Ok(users)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn delete_vault_by_id(&self, vault_id: &VaultIdentifier) -> Result<(), Error> {
        let InternalVault { vault, .. } = self.storage.vault(vault_id).await?;

        let keys = self.keys();
        let nostr_public_identifier: NostrPublicIdentifier = vault.nostr_public_identifier(keys);

        let filter: Filter = Filter::new()
            .kind(VAULT_KIND_V2)
            .author(keys.public_key())
            .identifier(nostr_public_identifier.to_string())
            .limit(1);
        let res: Vec<Event> = self
            .client
            .database()
            .query(vec![filter], Order::Desc)
            .await?;
        let vault_event: &Event = res.first().ok_or(Error::NotFound)?;

        let event = self.client.database().event_by_id(vault_event.id).await?;
        let author = event.author();
        if author == keys.public_key() {
            // Delete policy
            let builder = EventBuilder::new(Kind::EventDeletion, "", [Tag::event(vault_event.id)]);
            self.client.send_event_builder(builder).await?;

            self.storage.delete_vault(vault_id).await;

            // Unload policy
            self.manager.unload_policy(*vault_id).await?;

            // Update subscription filters
            self.set_resubscribe_vaults(true);

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }
}
