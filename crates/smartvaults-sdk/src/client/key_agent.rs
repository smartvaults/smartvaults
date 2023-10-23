// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::{Event, EventBuilder, EventId, Keys};
use smartvaults_protocol::v1::{SignerOffering, SmartVaultsEventBuilder};

use super::{Error, SmartVaults};

impl SmartVaults {
    pub async fn signer_offering(
        &self,
        id: String,
        offering: SignerOffering,
    ) -> Result<EventId, Error> {
        // Get keys
        let keys: Keys = self.keys().await;

        // Compose event
        let event: Event = EventBuilder::signer_offering(&keys, id, offering)?;

        // Publish event
        let event_id: EventId = self.send_event(event).await?;

        // Save to db
        // self.db.save_label(identifier, policy_id, label).await?; TODO

        Ok(event_id)
    }
}
