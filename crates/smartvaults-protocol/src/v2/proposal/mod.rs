// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Proposals

use nostr::{Event, EventBuilder, Keys, Tag};
use prost::Message;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::{Address, Network, Transaction};
use smartvaults_core::crypto::hash;
use smartvaults_core::miniscript::Descriptor;

mod proto;

use super::constants::PROPOSAL_KIND_V2;
use super::core::{ProtocolEncoding, ProtocolEncryption, SchemaVersion};
use super::{Error, Vault};
use crate::v2::proto::proposal::ProtoProposal;

/// Address recipient
pub struct Recipient {
    /// Address
    pub address: Address,
    /// Amount in SAT
    pub amount: u64,
}

/// Proposal type
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProposalType {
    /// Spending
    Spending,
    /// Proof of reserve
    ProofOfReserve,
    /// Key Agent Payment
    KeyAgentPayment,
}

/// Proposal
pub struct Proposal {
    /// Status
    pub status: ProposalStatus,
    /// Network
    pub network: Network,
}

impl Proposal {
    /// Get [`ProposalType`]
    pub fn r#type(&self) -> ProposalType {
        match &self.status {
            ProposalStatus::Pending(p) => match p {
                PendingProposal::Spending { .. } => ProposalType::Spending,
                PendingProposal::ProofOfReserve { .. } => ProposalType::ProofOfReserve,
                PendingProposal::KeyAgentPayment { .. } => ProposalType::KeyAgentPayment,
            },
            ProposalStatus::Completed(p) => match p {
                CompletedProposal::Spending { .. } => ProposalType::Spending,
                CompletedProposal::ProofOfReserve { .. } => ProposalType::ProofOfReserve,
                CompletedProposal::KeyAgentPayment { .. } => ProposalType::KeyAgentPayment,
            },
        }
    }

    /// Generate unique deterministic identifier
    ///
    /// WARNING: the deterministic identifier it's generated using the TXID
    /// so if the TX inside the PSBT change, the deterministic identifer will be different.
    pub fn generate_identifier(&self) -> String {
        // Extract TX
        let tx: &Transaction = match &self.status {
            ProposalStatus::Pending(inner) => match inner {
                PendingProposal::Spending { psbt, .. } => &psbt.unsigned_tx,
                PendingProposal::ProofOfReserve { psbt, .. } => &psbt.unsigned_tx,
                PendingProposal::KeyAgentPayment { psbt, .. } => &psbt.unsigned_tx,
            },
            ProposalStatus::Completed(inner) => match inner {
                CompletedProposal::Spending { tx, .. } => tx,
                CompletedProposal::ProofOfReserve { psbt, .. } => &psbt.unsigned_tx,
                CompletedProposal::KeyAgentPayment { tx, .. } => tx,
            },
        };

        let unhashed_identifier: String = format!("{}:{}", self.network.magic(), tx.txid());
        let hash: String = hash::sha256(unhashed_identifier).to_string();
        hash[..32].to_string()
    }
}

/// Proposal status
pub enum ProposalStatus {
    /// Pending proposal
    Pending(PendingProposal),
    /// Completed proposal
    Completed(CompletedProposal),
}

/// Pending proposal
pub enum PendingProposal {
    /// Spending
    Spending {
        /// Descriptor
        descriptor: Descriptor<String>,
        /// Recipients
        addresses: Vec<Recipient>,
        /// Description/note
        description: String,
        /// PSBT
        psbt: PartiallySignedTransaction,
    },
    /// Proof of reserve
    ProofOfReserve {
        /// PSBT
        psbt: PartiallySignedTransaction,
    },
    /// Key Agent Payment
    KeyAgentPayment {
        /// Descriptor
        descriptor: Descriptor<String>,
        /// Signer descriptor
        ///
        /// Needed to indentify the Key Agent and the signer used
        signer_descriptor: Descriptor<String>,
        /// Recipient
        recipient: Recipient,
        /// Description/note
        description: String,
        // period: Period,
        /// PSBT
        psbt: PartiallySignedTransaction,
    },
}

/// Completed proposal
pub enum CompletedProposal {
    /// Spending
    Spending {
        /// TX
        tx: Transaction,
    },
    /// Proof of reserve
    ProofOfReserve {
        /// PSBT
        psbt: PartiallySignedTransaction,
    },
    /// Key Agent Payment
    KeyAgentPayment {
        /// TX
        tx: Transaction,
    },
}

impl ProtocolEncoding for Proposal {
    type Err = Error;

    fn pre_encoding(&self) -> (SchemaVersion, Vec<u8>) {
        let proposal: ProtoProposal = self.into();
        (SchemaVersion::ProtoBuf, proposal.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoProposal = ProtoProposal::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for Proposal {
    type Err = Error;
}

/// Build [`Proposal`] event
pub fn build_event(vault: &Vault, proposal: &Proposal) -> Result<Event, Error> {
    // Keys
    let keys: Keys = Keys::new(vault.shared_key());

    // Encrypt
    let encrypted_content: String = proposal.encrypt_with_keys(&keys)?;

    // Compose and build event
    let identifier = Tag::Identifier(proposal.generate_identifier());
    Ok(EventBuilder::new(PROPOSAL_KIND_V2, encrypted_content, &[identifier]).to_event(&keys)?)
}
