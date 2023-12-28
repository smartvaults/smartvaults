// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Approval

use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::Network;

mod proto;

use super::VaultIdentifier;

/// Approval type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApprovalType {
    /// Spending
    Spending,
    /// Proof of Reserve
    ProofOfReserve,
    /// Key Agent payment
    KeyAgentPayment,
}

/// Approval version
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    /// V1
    #[default]
    V1 = 0x01,
}

/// Approval
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Approval {
    vault_id: VaultIdentifier,
    version: Version,
    psbt: PartiallySignedTransaction,
    r#type: ApprovalType,
    network: Network,
}

impl Approval {
    /// Compose new [`Approval`]
    pub fn new(
        vault_id: VaultIdentifier,
        psbt: PartiallySignedTransaction,
        r#type: ApprovalType,
        network: Network,
    ) -> Self {
        Self {
            vault_id,
            version: Version::default(),
            psbt,
            r#type,
            network,
        }
    }

    /// Vault Identifier
    pub fn vault_id(&self) -> VaultIdentifier {
        self.vault_id
    }

    /// Get PSBT
    pub fn psbt(&self) -> PartiallySignedTransaction {
        self.psbt.clone()
    }

    /// Get approval type
    pub fn r#type(&self) -> ApprovalType {
        self.r#type
    }

    /// Get approval network
    pub fn network(&self) -> Network {
        self.network
    }
}
