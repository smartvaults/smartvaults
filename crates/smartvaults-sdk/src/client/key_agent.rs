// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::{Event, EventBuilder, EventId, Keys};
use smartvaults_protocol::v1::{SignerOffering, SmartVaultsEventBuilder};

use crate::types::{KeyAgent, User};

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
        self.send_event(event).await
    }

    /// Get key agents
    pub async fn key_agents(&self) -> Result<Vec<KeyAgent>, Error> {
        let agents = self.key_agents.read().await;
        let mut list = Vec::with_capacity(agents.len());
        for (public_key, set) in agents.clone().into_iter() {
            let metadata = self.get_public_key_metadata(public_key).await?;
            list.push(KeyAgent {
                user: User::new(public_key, metadata),
                list: set,
            })
        }
        Ok(list)
    }
}
