// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::EventId;
use smartvaults_protocol::v1::Label;
use smartvaults_protocol::v2::VaultIdentifier;

use super::{Error, SmartVaults};
use crate::storage::InternalVault;

impl SmartVaults {
    pub async fn save_label(
        &self,
        vault_id: &VaultIdentifier,
        _label: Label,
    ) -> Result<EventId, Error> {
        let InternalVault { .. } = self.storage.vault(vault_id).await?;

        // TODO: check if address or UTXO actually belong to the policy

        todo!();

        // Compose event
        // let event: Event = EventBuilder::label(&shared_key, policy_id, &label, &public_keys)?;
        //
        // Publish event
        // let event_id: EventId = self.client.send_event(event).await?;
        //
        // Save to db
        // let identifier: String = label.generate_identifier(&shared_key)?;
        // self.storage.save_label(identifier, policy_id, label).await;

        // Ok(event_id)
    }
}
