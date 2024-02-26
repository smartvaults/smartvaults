// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Proposals

use core::fmt;
use std::cmp::Ordering;

use nostr::{Event, EventBuilder, Keys, Tag, Timestamp};
use prost::Message;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::{Network, Transaction};
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::{
    Destination, ProofOfReserveProposal, ProposalSigning, Recipient, Seed, SpendingProposal,
};

pub mod id;
mod proto;

pub use self::id::ProposalIdentifier;
use super::constants::PROPOSAL_KIND_V2;
use super::message::{MessageVersion, ProtocolEncoding, ProtocolEncryption};
use super::{Approval, ApprovalType, Error, Vault, VaultIdentifier};
use crate::v2::proto::proposal::ProtoProposal;

/// Period
///
/// From UNIX timestamo to UNIX timestamp
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Period {
    /// From timestamp
    pub from: Timestamp,
    /// To timestamp
    pub to: Timestamp,
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

impl fmt::Display for ProposalType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spending => write!(f, "Spending"),
            Self::ProofOfReserve => write!(f, "Proof of Reserve"),
            Self::KeyAgentPayment => write!(f, "Key Agent Payment"),
        }
    }
}

/// Proposal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Proposal {
    vault_id: VaultIdentifier,
    status: ProposalStatus,
    network: Network,
    timestamp: Timestamp,
    description: Option<String>,
}

impl PartialOrd for Proposal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Proposal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.timestamp.cmp(&other.timestamp)
    }
}

impl Proposal {
    /// Create new pending proposal
    pub fn pending(vault_id: VaultIdentifier, proposal: PendingProposal, network: Network) -> Self {
        Self {
            vault_id,
            status: ProposalStatus::Pending(proposal),
            network,
            timestamp: Timestamp::now(),
            description: None,
        }
    }

    /// Compute unique deterministic identifier
    ///
    /// WARNING: the deterministic identifier it's generated using the TXID
    /// so if the TX inside the PSBT change, the deterministic identifer will be different!
    pub fn compute_id(&self) -> ProposalIdentifier {
        ProposalIdentifier::from((self.network, self.tx()))
    }

    /// Vault Identifier
    pub fn vault_id(&self) -> VaultIdentifier {
        self.vault_id
    }

    /// Proposal status
    pub fn status(&self) -> &ProposalStatus {
        &self.status
    }

    /// Network
    pub fn network(&self) -> Network {
        self.network
    }

    /// Last UNIX timestamp update
    pub fn timestamp(&self) -> Timestamp {
        self.timestamp
    }

    /// Check if [`Proposal`] is finalized/completed
    pub fn is_finalized(&self) -> bool {
        matches!(self.status, ProposalStatus::Completed(..))
    }

    /// Check if [`Proposal`] is broadcastable
    ///
    /// Internally check if is finalized and if type is different from `ProposalType::ProofOfReserve`.
    ///
    /// Return `false` if status is `pending` or type is `ProposalType::ProofOfReserve`
    pub fn is_broadcastable(&self) -> bool {
        match &self.status {
            ProposalStatus::Pending(..) => false,
            ProposalStatus::Completed(p) => matches!(
                p,
                CompletedProposal::Spending { .. } | CompletedProposal::KeyAgentPayment { .. }
            ),
        }
    }

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

    /// Extract TX
    pub fn tx(&self) -> &Transaction {
        match &self.status {
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
        }
    }

    /// Get PSBT if status is `ProposalStatus::Pending`
    pub fn psbt(&self) -> Option<&PartiallySignedTransaction> {
        match &self.status {
            ProposalStatus::Pending(p) => match p {
                PendingProposal::Spending { psbt, .. } => Some(psbt),
                PendingProposal::ProofOfReserve { psbt, .. } => Some(psbt),
                PendingProposal::KeyAgentPayment { psbt, .. } => Some(psbt),
            },
            ProposalStatus::Completed(..) => None,
        }
    }

    /// Get description
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Change proposal description
    pub fn change_description<S>(&mut self, description: S)
    where
        S: Into<String>,
    {
        self.description = Some(description.into())
    }

    /// Approve a **pending** proposal with [`Seed`]
    ///
    /// If the proposal is already completed, will return `Error::ProposalAlreadyFinalized`.
    pub fn approve(&self, seed: &Seed) -> Result<Approval, Error> {
        if let ProposalStatus::Pending(pending) = &self.status {
            let (psbt, r#type): (PartiallySignedTransaction, ApprovalType) = match pending {
                PendingProposal::Spending {
                    descriptor, psbt, ..
                } => {
                    let spending = SpendingProposal {
                        descriptor: descriptor.clone(),
                        psbt: psbt.clone(),
                        network: self.network,
                    };
                    (spending.approve(seed, Vec::new())?, ApprovalType::Spending)
                }
                PendingProposal::KeyAgentPayment {
                    descriptor, psbt, ..
                } => {
                    let spending = SpendingProposal {
                        descriptor: descriptor.clone(),
                        psbt: psbt.clone(),
                        network: self.network,
                    };
                    (
                        spending.approve(seed, Vec::new())?,
                        ApprovalType::KeyAgentPayment,
                    )
                }
                PendingProposal::ProofOfReserve {
                    descriptor,
                    message,
                    psbt,
                } => {
                    let proof_of_reserve = ProofOfReserveProposal {
                        descriptor: descriptor.clone(),
                        message: message.to_owned(),
                        psbt: psbt.clone(),
                        network: self.network,
                    };
                    (
                        proof_of_reserve.approve(seed, Vec::new())?,
                        ApprovalType::ProofOfReserve,
                    )
                }
            };

            Ok(Approval::new(
                self.vault_id,
                self.compute_id(),
                psbt,
                r#type,
                self.network,
            ))
        } else {
            Err(Error::ProposalAlreadyFinalized)
        }
    }

    /// Try to finalize without update internal status (useful to check if is signed)
    pub fn try_finalize<I>(&self, approvals: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Approval>,
    {
        let mut proposal = self.clone();
        proposal.finalize(approvals)
    }

    /// Finalize the [`Proposal`] and update the status to `ProposalStatus::Completed`.
    ///
    /// If the proposal is already completed, will return `Error::ProposalAlreadyFinalized`.
    pub fn finalize<I>(&mut self, approvals: I) -> Result<(), Error>
    where
        I: IntoIterator<Item = Approval>,
    {
        if let ProposalStatus::Pending(pending) = &self.status {
            let psbts = approvals.into_iter().map(|a| a.psbt());
            match pending {
                PendingProposal::Spending {
                    descriptor, psbt, ..
                } => {
                    let spending = SpendingProposal {
                        descriptor: descriptor.clone(),
                        psbt: psbt.clone(),
                        network: self.network,
                    };
                    let tx = spending.finalize(psbts)?;
                    self.status = ProposalStatus::Completed(CompletedProposal::Spending { tx });
                }
                PendingProposal::KeyAgentPayment {
                    descriptor, psbt, ..
                } => {
                    let spending = SpendingProposal {
                        descriptor: descriptor.clone(),
                        psbt: psbt.clone(),
                        network: self.network,
                    };
                    let tx = spending.finalize(psbts)?;
                    self.status =
                        ProposalStatus::Completed(CompletedProposal::KeyAgentPayment { tx });
                }
                PendingProposal::ProofOfReserve {
                    descriptor,
                    message,
                    psbt,
                } => {
                    let proof_of_reserve = ProofOfReserveProposal {
                        descriptor: descriptor.clone(),
                        message: message.to_owned(),
                        psbt: psbt.clone(),
                        network: self.network,
                    };
                    let proof = proof_of_reserve.finalize(psbts)?;
                    self.status = ProposalStatus::Completed(CompletedProposal::ProofOfReserve {
                        descriptor: descriptor.clone(),
                        message: message.clone(),
                        psbt: proof.psbt,
                    });
                }
            }

            // Update timestamp
            self.timestamp = Timestamp::now();

            Ok(())
        } else {
            Err(Error::ProposalAlreadyFinalized)
        }
    }
}

/// Proposal status
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProposalStatus {
    /// Pending proposal
    Pending(PendingProposal),
    /// Completed proposal
    Completed(CompletedProposal),
}

impl fmt::Display for ProposalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pending(..) => write!(f, "Pending"),
            Self::Completed(..) => write!(f, "Completed"),
        }
    }
}

/// Pending proposal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PendingProposal {
    /// Spending
    Spending {
        /// Descriptor
        descriptor: Descriptor<String>,
        /// Recipients
        destination: Destination,
        /// PSBT
        psbt: PartiallySignedTransaction,
    },
    /// Proof of reserve
    ProofOfReserve {
        /// Descriptor
        descriptor: Descriptor<String>,
        /// Message
        message: String,
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
        /// Period
        period: Period,
        /// PSBT
        psbt: PartiallySignedTransaction,
    },
}

/// Completed proposal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CompletedProposal {
    /// Spending
    Spending {
        /// TX
        tx: Transaction,
    },
    /// Proof of reserve
    ProofOfReserve {
        /// Descriptor
        descriptor: Descriptor<String>,
        /// Message
        message: String,
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

    fn pre_encoding(&self) -> (MessageVersion, Vec<u8>) {
        let proposal: ProtoProposal = self.into();
        (MessageVersion::ProtoBuf, proposal.encode_to_vec())
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
    let keys: Keys = Keys::new(vault.shared_key().clone());

    // Encrypt
    let encrypted_content: String = proposal.encrypt_with_keys(&keys)?;

    // Compose and build event
    let identifier = Tag::Identifier(proposal.compute_id().to_string());
    Ok(EventBuilder::new(PROPOSAL_KIND_V2, encrypted_content, [identifier]).to_event(&keys)?)
}
