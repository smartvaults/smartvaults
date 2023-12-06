// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Proposals

use std::cmp::Ordering;

use nostr::{Event, EventBuilder, Keys, Tag, Timestamp};
use prost::Message;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::{Address, Network, Transaction};
use smartvaults_core::crypto::hash;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::{ProofOfReserveProposal, ProposalSigning, Seed, SpendingProposal};

mod proto;

use super::constants::PROPOSAL_KIND_V2;
use super::core::{ProtocolEncoding, ProtocolEncryption, SchemaVersion};
use super::{Approval, Error, Vault};
use crate::v2::proto::proposal::ProtoProposal;

/// Address recipient
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Recipient {
    /// Address
    pub address: Address,
    /// Amount in SAT
    pub amount: u64,
}

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

/// Proposal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Proposal {
    /// Status
    pub status: ProposalStatus,
    /// Network
    pub network: Network,
    /// Last update UNIX timestamp
    pub timestamp: Timestamp,
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

    /// Approve a **pending** proposal
    pub fn approve(&self, seed: &Seed) -> Result<PartiallySignedTransaction, Error> {
        if let ProposalStatus::Pending(pending) = &self.status {
            match pending {
                PendingProposal::Spending {
                    descriptor, psbt, ..
                } => {
                    let spending = SpendingProposal {
                        descriptor: descriptor.clone(),
                        psbt: psbt.clone(),
                        network: self.network,
                    };
                    Ok(spending.approve(seed, Vec::new())?)
                }
                PendingProposal::KeyAgentPayment {
                    descriptor, psbt, ..
                } => {
                    let spending = SpendingProposal {
                        descriptor: descriptor.clone(),
                        psbt: psbt.clone(),
                        network: self.network,
                    };
                    Ok(spending.approve(seed, Vec::new())?)
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
                    Ok(proof_of_reserve.approve(seed, Vec::new())?)
                }
            }
        } else {
            Err(Error::ProposalAlreadyFinalized)
        }
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
                    Ok(())
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
                    Ok(())
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
                        psbt: proof.psbt,
                    });
                    Ok(())
                }
            }
        } else {
            Err(Error::ProposalAlreadyFinalized)
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
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ProposalStatus {
    /// Pending proposal
    Pending(PendingProposal),
    /// Completed proposal
    Completed(CompletedProposal),
}

/// Pending proposal
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
        /// Description/note
        description: String,
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
