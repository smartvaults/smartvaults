// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::{secp256k1::XOnlyPublicKey, Event, EventBuilder, EventId, Keys};
use smartvaults_protocol::v1::{SignerOffering, SmartVaultsEventBuilder};

use super::{Error, SmartVaults};
use crate::types::{KeyAgent, User};

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

    /// Get Key Agents
    pub async fn key_agents(&self) -> Result<Vec<KeyAgent>, Error> {
        let agents = self.key_agents.read().await;
        let mut list = Vec::with_capacity(agents.len());
        for (public_key, set) in agents.clone().into_iter() {
            let metadata = self.get_public_key_metadata(public_key).await?;
            list.push(KeyAgent {
                user: User::new(public_key, metadata),
                list: set,
                verified: false, // TODO: check if verified
            })
        }
        Ok(list)
    }

    /// Request signers to Key Agent
    pub async fn request_signers_to_key_agent(
        &self,
        key_agent: XOnlyPublicKey,
    ) -> Result<(), Error> {
        self.add_contact(key_agent).await?;
        Ok(())
    }
}
