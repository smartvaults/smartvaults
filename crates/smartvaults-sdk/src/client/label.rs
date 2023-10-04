// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::{Event, EventBuilder, EventId, Keys};
use smartvaults_protocol::v1::{Label, SmartVaultsEventBuilder};

use super::{Error, SmartVaults};
use crate::storage::InternalVault;

impl SmartVaults {
    pub async fn save_label(&self, policy_id: EventId, label: Label) -> Result<EventId, Error> {
        let shared_key: Keys = self.storage.shared_key(&policy_id).await?;
        let InternalVault { public_keys, .. } = self.storage.vault(&policy_id).await?;

        // TODO: check if address or UTXO actually belong to the policy

        // Compose event
        let event: Event = EventBuilder::label(&shared_key, policy_id, &label, &public_keys)?;

        // Publish event
        let event_id: EventId = self.client.send_event(event).await?;

        // Save to db
        let identifier: String = label.generate_identifier(&shared_key)?;
        self.storage.save_label(identifier, policy_id, label).await;

        Ok(event_id)
    }
}
