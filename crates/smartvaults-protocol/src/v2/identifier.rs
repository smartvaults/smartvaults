// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use serde::{Deserialize, Serialize};
use smartvaults_core::bitcoin::hashes::sha256::Hash as Sha256Hash;

/// Deterministic identifier
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Identifier {
    inner: Sha256Hash,
}
