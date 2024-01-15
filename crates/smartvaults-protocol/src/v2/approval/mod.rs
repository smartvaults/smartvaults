// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Approval

use nostr::{Event, EventBuilder, Keys, Tag, Timestamp};
use prost::Message;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::Network;

mod proto;

use super::constants::{APPROVAL_KIND_V2, WRAPPER_EXIPRATION};
use super::{ProposalIdentifier, ProtocolEncoding, ProtocolEncryption, Vault, VaultIdentifier};
use crate::v2::message::EncodingVersion;
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

    fn protocol_network(&self) -> Network {
        self.network
    }

    fn pre_encoding(&self) -> (EncodingVersion, Vec<u8>) {
        let proposal: ProtoApproval = self.into();
        (EncodingVersion::ProtoBuf, proposal.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoApproval = ProtoApproval::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for Approval {
    type Err = Error;
}

/// Build [`Approval`] event
pub fn build_event(vault: &Vault, approval: &Approval, keys: &Keys) -> Result<Event, Error> {
    let shared_key: Keys = Keys::new(vault.shared_key());
    let encrypted_content: String = approval.encrypt_with_keys(&shared_key)?;

    // Compose and build event
    Ok(EventBuilder::new(
        APPROVAL_KIND_V2,
        encrypted_content,
        [
            Tag::public_key(shared_key.public_key()),
            Tag::Expiration(Timestamp::now() + WRAPPER_EXIPRATION),
        ],
    )
    .to_event(keys)?)
}
