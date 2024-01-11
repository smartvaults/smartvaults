// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::{Event, Keys};
use smartvaults_core::{Policy, PolicyTemplate};
use smartvaults_protocol::v2::{self, Vault, VaultIdentifier};

use super::{Error, SmartVaults};
use crate::types::GetVault;

impl SmartVaults {
    /// Get own vaults
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn vaults(&self) -> Result<Vec<GetVault>, Error> {
        let items = self.storage.vaults().await;
        let mut vaults: Vec<GetVault> = Vec::with_capacity(items.len());

        for (id, vault) in items.into_iter() {
            vaults.push(GetVault {
                vault,
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
        Ok(GetVault {
            vault: self.storage.vault(vault_id).await?,
            balance: self.manager.get_balance(vault_id).await?,
            last_sync: self.manager.last_sync(vault_id).await?,
        })
    }

    pub async fn save_vault<S, D>(
        &self,
        _name: S,
        _description: S,
        descriptor: D,
    ) -> Result<VaultIdentifier, Error>
    where
        S: Into<String>,
        D: AsRef<str>,
    {
        let descriptor: String = descriptor.into();

        // Generate a shared key
        let shared_key = Keys::generate();
        let vault = Vault::new(descriptor, self.network, shared_key.secret_key()?)?;

        // Compose event
        let keys = self.keys();
        let event: Event = v2::vault::build_event(keys, &vault)?;

        // Publish event
        self.client.send_event(event).await?;

        let vault_id: VaultIdentifier = vault.id();
        let policy: Policy = vault.policy();

        // Index event
        self.storage.save_vault(vault_id, vault).await;

        // Load policy
        self.manager.load_policy(vault_id, policy).await?;

        Ok(vault_id)
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
}
