// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;

use bdk::bitcoin::{OutPoint, Script};
use nostr_sdk::EventId;

use super::{Error, Store};
use crate::types::{Label, LabelData, LabelKind};
use crate::util::encryption::EncryptionWithKeys;

impl Store {
    pub fn save_label(
        &self,
        identifier: String,
        policy_id: EventId,
        label: Label,
    ) -> Result<(), Error> {
        let conn = self.pool.get()?;
        let kind: LabelKind = label.kind();
        let label: String = label.encrypt_with_keys(&self.keys)?;
        conn.execute(
            "INSERT INTO labels (id, policy_id, kind, label) VALUES (?, ?, ?, ?) ON CONFLICT(id) DO UPDATE SET label = ?;",
            (
                identifier,
                policy_id.to_hex(),
                kind.to_string(),
                label.clone(),
                label,
            ),
        )?;
        Ok(())
    }

    pub fn get_addresses_labels(
        &self,
        policy_id: EventId,
    ) -> Result<HashMap<Script, Label>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT id, label FROM labels WHERE policy_id = ? AND kind = ?;")?;
        let mut rows = stmt.query((policy_id.to_hex(), LabelKind::Address.to_string()))?;
        let mut labels = HashMap::new();
        while let Ok(Some(row)) = rows.next() {
            let label: String = row.get(0)?;
            let label = Label::decrypt_with_keys(&self.keys, label)?;
            if let LabelData::Address(addr) = label.data() {
                labels.insert(addr.script_pubkey(), label);
            };
        }
        Ok(labels)
    }

    pub fn get_utxos_labels(&self, policy_id: EventId) -> Result<HashMap<OutPoint, Label>, Error> {
        let conn = self.pool.get()?;
        let mut stmt =
            conn.prepare_cached("SELECT label FROM labels WHERE policy_id = ? AND kind = ?;")?;
        let mut rows = stmt.query((policy_id.to_hex(), LabelKind::Utxo.to_string()))?;
        let mut labels = HashMap::new();
        while let Ok(Some(row)) = rows.next() {
            let label: String = row.get(0)?;
            let label = Label::decrypt_with_keys(&self.keys, label)?;
            if let LabelData::Utxo(utxo) = label.data() {
                labels.insert(utxo, label);
            };
        }
        Ok(labels)
    }
}
