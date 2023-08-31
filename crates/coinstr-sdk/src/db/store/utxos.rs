// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::{collections::HashSet, str::FromStr};

use coinstr_core::bitcoin::hashes::sha256::Hash as Sha256Hash;
use coinstr_core::bitcoin::hashes::Hash;
use coinstr_core::bitcoin::OutPoint;
use nostr_sdk::EventId;

use super::{Error, Store};

impl Store {
    pub async fn freeze_utxo(
        &self,
        utxo: OutPoint,
        policy_id: EventId,
        proposal_id: Option<EventId>,
    ) -> Result<(), Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let utxo = Sha256Hash::hash(utxo.to_string().as_bytes());
        conn.execute(
            "INSERT OR IGNORE INTO frozen_utxos (utxo_hash, policy_id, proposal_id) VALUES (?, ?, ?);",
            (
                utxo.to_string(),
                policy_id.to_hex(),
                proposal_id.map(|p| p.to_hex())
            ),
        )?;
        Ok(())
        }).await?
    }

    pub async fn get_frozen_utxos(&self, policy_id: EventId) -> Result<HashSet<Sha256Hash>, Error> {
        let conn = self.acquire().await?;
        conn.interact(move |conn| {
            let mut stmt =
                conn.prepare_cached("SELECT utxo_hash FROM frozen_utxos WHERE policy_id = ?;")?;
            let mut rows = stmt.query([policy_id.to_hex()])?;
            let mut utxos = HashSet::new();
            while let Ok(Some(row)) = rows.next() {
                let utxo_hash: String = row.get(0)?;
                utxos.insert(Sha256Hash::from_str(&utxo_hash)?);
            }
            Ok(utxos)
        })
        .await?
    }
}
