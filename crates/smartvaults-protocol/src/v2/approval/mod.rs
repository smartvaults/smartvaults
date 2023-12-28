// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Approval

use prost::Message;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::Network;

mod proto;

use super::{ProposalIdentifier, ProtocolEncoding, ProtocolEncryption, VaultIdentifier};
use crate::v2::core::SchemaVersion;
use crate::v2::proto::approval::ProtoApproval;
use crate::v2::Error;

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
    proposal_id: ProposalIdentifier,
    version: Version,
    psbt: PartiallySignedTransaction,
    r#type: ApprovalType,
    network: Network,
}

impl Approval {
    /// Compose new [`Approval`]
    pub fn new(
        vault_id: VaultIdentifier,
        proposal_id: ProposalIdentifier,
        psbt: PartiallySignedTransaction,
        r#type: ApprovalType,
        network: Network,
    ) -> Self {
        Self {
            vault_id,
            proposal_id,
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

    /// Proposal Identifier
    pub fn proposal_id(&self) -> ProposalIdentifier {
        self.proposal_id
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

impl ProtocolEncoding for Approval {
    type Err = Error;

    fn pre_encoding(&self) -> (SchemaVersion, Vec<u8>) {
        let proposal: ProtoApproval = self.into();
        (SchemaVersion::ProtoBuf, proposal.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoApproval = ProtoApproval::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for Approval {
    type Err = Error;
}
