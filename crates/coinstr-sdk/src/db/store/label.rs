// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;

use bdk::bitcoin::{Address, OutPoint};
use nostr_sdk::EventId;

use super::{Error, Store};
use crate::types::{Label, LabelKind};
use crate::util::encryption::EncryptionWithKeys;

impl Store {
    pub fn save_label(
        &self,
        identifier: String,
        policy_id: EventId,
        label: Label,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let type_string: String = label.kind().type_string();
        let label: String = label.encrypt_with_keys(&self.keys)?;
        conn.execute(
            "INSERT INTO labels (id, policy_id, type, label) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET label = ?;",
            (
                identifier,
                policy_id.to_hex(),
                type_string,
                label.clone(),
                label,
            ),
        )?;
        Ok(())
    }

    pub fn get_addresses_labels(
        &self,
        policy_id: EventId,
    ) -> Result<HashMap<Address, Label>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT id, label FROM labels WHERE policy_id = ? AND type = ?;")?;
        let mut rows = stmt.query((policy_id.to_hex(), "address"))?;
        let mut labels = HashMap::new();
        while let Ok(Some(row)) = rows.next() {
            let label: String = row.get(0)?;
            let label = Label::decrypt_with_keys(&self.keys, label)?;
            if let LabelKind::Address(addr) = label.kind() {
                labels.insert(addr, label);
            };
        }
        Ok(labels)
    }

    pub fn get_utxos_labels(&self, policy_id: EventId) -> Result<HashMap<OutPoint, Label>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT label FROM labels WHERE policy_id = ? AND type = ?;")?;
        let mut rows = stmt.query((policy_id.to_hex(), "utxo"))?;
        let mut labels = HashMap::new();
        while let Ok(Some(row)) = rows.next() {
            let label: String = row.get(0)?;
            let label = Label::decrypt_with_keys(&self.keys, label)?;
            if let LabelKind::Utxo(utxo) = label.kind() {
                labels.insert(utxo, label);
            };
        }
        Ok(labels)
    }
}
