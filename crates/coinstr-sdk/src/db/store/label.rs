// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;

use nostr_sdk::EventId;

use super::{Error, Store};
use crate::types::Label;
use crate::util::encryption::EncryptionWithKeys;

impl Store {
    pub fn save_label(
        &self,
        identifier: String,
        policy_id: EventId,
        label: Label,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let label: String = label.encrypt_with_keys(&self.keys)?;
        conn.execute(
            "INSERT INTO labels (id, policy_id, label) VALUES (?, ?, ?) ON CONFLICT(id) DO UPDATE SET label = ?;",
            (
                identifier,
                policy_id.to_hex(),
                label.clone(),
                label,
            ),
        )?;
        Ok(())
    }

    pub fn get_labels(&self, policy_id: EventId) -> Result<HashMap<String, Label>, Error> {
        let conn = self.pool.get()?;
        let mut stmt = conn.prepare_cached("SELECT id, label FROM labels WHERE policy_id = ?;")?;
        let mut rows = stmt.query([policy_id.to_hex()])?;
        let mut labels = HashMap::new();
        while let Ok(Some(row)) = rows.next() {
            let id: String = row.get(0)?;
            let label: String = row.get(1)?;
            labels.insert(id, Label::decrypt_with_keys(&self.keys, label)?);
        }
        Ok(labels)
    }
}
