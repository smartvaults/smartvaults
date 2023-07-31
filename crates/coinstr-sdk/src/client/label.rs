// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use nostr_sdk::{Event, EventBuilder, EventId, Keys, Tag};

use super::{Coinstr, Error};
use crate::constants::{LABELS_KIND, SEND_TIMEOUT};
use crate::types::Label;
use crate::util::encryption::EncryptionWithKeys;

impl Coinstr {
    pub async fn save_label(&self, policy_id: EventId, label: Label) -> Result<(), Error> {
        let shared_key: Keys = self.db.get_shared_key(policy_id)?;

        // TODO: check if address or UTXO actually belong to the policy

        // Compose label event
        let identifier: String = label.generate_identifier(&shared_key)?;
        let content: String = label.encrypt_with_keys(&shared_key)?;
        let tags: Vec<Tag> = vec![
            Tag::Identifier(identifier),
            Tag::Event(policy_id, None, None),
        ];
        let event: Event = EventBuilder::new(LABELS_KIND, content, &tags).to_event(&shared_key)?;

        // Publish event
        self.send_event(event, Some(SEND_TIMEOUT)).await?;

        // TODO: save in db

        Ok(())
    }
}
