// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::{Event, EventBuilder, EventId, Keys};
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_protocol::v1::{Label, SmartVaultsEventBuilder};

use super::{Error, SmartVaults};

impl SmartVaults {
    pub async fn save_label(&self, policy_id: EventId, label: Label) -> Result<EventId, Error> {
        let shared_key: Keys = self.db.get_shared_key(policy_id).await?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;

        // TODO: check if address or UTXO actually belong to the policy

        // Compose event
        let event: Event = EventBuilder::label(&shared_key, policy_id, &label, &nostr_pubkeys)?;

        // Publish event
        let event_id: EventId = self.send_event(event).await?;

        // Save to db
        let identifier: String = label.generate_identifier(&shared_key)?;
        self.db.save_label(identifier, policy_id, label).await?;

        Ok(event_id)
    }
}
