// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use coinstr_core::secp256k1::XOnlyPublicKey;
use coinstr_protocol::v1::constants::LABELS_KIND;
use coinstr_protocol::v1::{Encryption, Label};
use nostr_sdk::{Event, EventBuilder, EventId, Keys, Tag};

use super::{Coinstr, Error};

impl Coinstr {
    pub async fn save_label(&self, policy_id: EventId, label: Label) -> Result<EventId, Error> {
        let shared_key: Keys = self.db.get_shared_key(policy_id).await?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;

        // TODO: check if address or UTXO actually belong to the policy

        // Compose event
        let identifier: String = label.generate_identifier(&shared_key)?;
        let content: String = label.encrypt_with_keys(&shared_key)?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Identifier(identifier.clone()));
        tags.push(Tag::Event(policy_id, None, None));
        let event: Event = EventBuilder::new(LABELS_KIND, content, &tags).to_event(&shared_key)?;

        // Publish event
        let event_id = self.send_event(event).await?;

        // Save to db
        self.db.save_label(identifier, policy_id, label).await?;

        Ok(event_id)
    }
}
