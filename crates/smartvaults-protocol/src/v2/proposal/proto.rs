// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use nostr::Timestamp;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::{consensus, Address, Amount, Network};
use smartvaults_core::hashes::Hash;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::{Destination, Recipient};

use super::{
    CompletedProposal, PendingProposal, Period, Proposal, ProposalIdentifier, ProposalStatus,
};
use crate::v2::proto::proposal::{
    ProtoCompletedKeyAgentPayment, ProtoCompletedProofOfReserve, ProtoCompletedProposal,
    ProtoCompletedProposalEnum, ProtoCompletedSpending, ProtoDestination, ProtoDestinationEnum,
    ProtoMultipleRecipients, ProtoPendingKeyAgentPayment, ProtoPendingProofOfReserve,
    ProtoPendingProposal, ProtoPendingProposalEnum, ProtoPendingSpending, ProtoPeriod,
    ProtoProposal, ProtoProposalIdentifier, ProtoProposalStatus, ProtoProposalStatusEnum,
    ProtoRecipient,
};
use crate::v2::proto::vault::ProtoVaultIdentifier;
use crate::v2::{Error, NetworkMagic, VaultIdentifier};

impl From<&ProposalIdentifier> for ProtoProposalIdentifier {
    fn from(id: &ProposalIdentifier) -> Self {
        Self {
            id: id.as_byte_array().to_vec(),
        }
    }
}

impl From<ProposalIdentifier> for ProtoProposalIdentifier {
    fn from(id: ProposalIdentifier) -> Self {
        Self {
            id: id.to_byte_array().to_vec(),
        }
    }
}

impl From<&Recipient> for ProtoRecipient {
    fn from(recipient: &Recipient) -> Self {
        ProtoRecipient {
            address: recipient.address.to_string(),
            amount: recipient.amount.to_sat(),
        }
    }
}

impl From<&Destination> for ProtoDestination {
    fn from(value: &Destination) -> Self {
        ProtoDestination {
            destination: Some(match value {
                Destination::Drain(address) => ProtoDestinationEnum::Drain(address.to_string()),
                Destination::Single(recipient) => ProtoDestinationEnum::Single(recipient.into()),
                Destination::Multiple(recipients) => {
                    ProtoDestinationEnum::Multiple(ProtoMultipleRecipients {
                        recipients: recipients.iter().map(|r| r.into()).collect(),
                    })
                }
            }),
        }
    }
}

impl From<&Period> for ProtoPeriod {
    fn from(period: &Period) -> Self {
        ProtoPeriod {
            from: period.from.as_u64(),
            to: period.to.as_u64(),
        }
    }
}

impl From<&PendingProposal> for ProtoPendingProposal {
    fn from(value: &PendingProposal) -> Self {
        Self {
            proposal: Some(match value {
                PendingProposal::Spending {
                    descriptor,
                    destination,
                    psbt,
                } => ProtoPendingProposalEnum::Spending(ProtoPendingSpending {
                    descriptor: descriptor.to_string(),
                    destination: Some(destination.into()),
                    psbt: psbt.to_string(),
                }),
                PendingProposal::ProofOfReserve {
                    descriptor,
                    message,
                    psbt,
                } => ProtoPendingProposalEnum::ProofOfReserve(ProtoPendingProofOfReserve {
                    descriptor: descriptor.to_string(),
                    message: message.to_owned(),
                    psbt: psbt.to_string(),
                }),
                PendingProposal::KeyAgentPayment {
                    descriptor,
                    signer_descriptor,
                    recipient,
                    period,
                    psbt,
                } => ProtoPendingProposalEnum::KeyAgentPayment(ProtoPendingKeyAgentPayment {
                    descriptor: descriptor.to_string(),
                    signer_descriptor: signer_descriptor.to_string(),
                    recipient: Some(recipient.into()),
                    period: Some(period.into()),
                    psbt: psbt.to_string(),
                }),
            }),
        }
    }
}

impl From<&CompletedProposal> for ProtoCompletedProposal {
    fn from(value: &CompletedProposal) -> Self {
        Self {
            proposal: Some(match value {
                CompletedProposal::Spending { tx } => {
                    ProtoCompletedProposalEnum::Spending(ProtoCompletedSpending {
                        tx: consensus::serialize(tx),
                    })
                }
                CompletedProposal::ProofOfReserve {
                    psbt,
                    descriptor,
                    message,
                } => ProtoCompletedProposalEnum::ProofOfReserve(ProtoCompletedProofOfReserve {
                    descriptor: descriptor.to_string(),
                    message: message.clone(),
                    psbt: psbt.to_string(),
                }),
                CompletedProposal::KeyAgentPayment { tx } => {
                    ProtoCompletedProposalEnum::KeyAgentPayment(ProtoCompletedKeyAgentPayment {
                        tx: consensus::serialize(tx),
                    })
                }
            }),
        }
    }
}

impl TryFrom<ProtoCompletedProposal> for CompletedProposal {
    type Error = Error;

    fn try_from(value: ProtoCompletedProposal) -> Result<Self, Self::Error> {
        match value
            .proposal
            .ok_or(Error::NotFound(String::from("completed proposal")))?
        {
            ProtoCompletedProposalEnum::Spending(inner) => Ok(Self::Spending {
                tx: consensus::deserialize(&inner.tx)?,
            }),
            ProtoCompletedProposalEnum::ProofOfReserve(inner) => Ok(Self::ProofOfReserve {
                descriptor: Descriptor::from_str(&inner.descriptor)?,
                message: inner.message,
                psbt: PartiallySignedTransaction::from_str(&inner.psbt)?,
            }),
            ProtoCompletedProposalEnum::KeyAgentPayment(inner) => Ok(Self::KeyAgentPayment {
                tx: consensus::deserialize(&inner.tx)?,
            }),
        }
    }
}

impl From<&ProposalStatus> for ProtoProposalStatusEnum {
    fn from(value: &ProposalStatus) -> Self {
        match value {
            ProposalStatus::Pending(pending) => Self::Pending(pending.into()),
            ProposalStatus::Completed(completed) => Self::Completed(completed.into()),
        }
    }
}

impl From<&Proposal> for ProtoProposal {
    fn from(proposal: &Proposal) -> Self {
        ProtoProposal {
            vault_id: Some(ProtoVaultIdentifier {
                id: proposal.vault_id.as_byte_array().to_vec(),
            }),
            status: Some(ProtoProposalStatus {
                proposal: Some((&proposal.status).into()),
            }),
            network: proposal.network.magic().to_bytes().to_vec(),
            timestamp: proposal.timestamp.as_u64(),
            description: proposal.description.clone(),
        }
    }
}

impl TryFrom<ProtoProposal> for Proposal {
    type Error = Error;

    fn try_from(value: ProtoProposal) -> Result<Self, Self::Error> {
        let vault_id: ProtoVaultIdentifier = value
            .vault_id
            .ok_or(Error::NotFound(String::from("vault identifier")))?;
        let vault_id: VaultIdentifier = VaultIdentifier::from_slice(&vault_id.id)?;
        let network: Network = NetworkMagic::from_slice(&value.network)?.into();
        let status = value
            .status
            .ok_or(Error::NotFound(String::from("proposal status")))?;
        let status = match status
            .proposal
            .ok_or(Error::NotFound(String::from("proposal status enum")))?
        {
            ProtoProposalStatusEnum::Pending(inner) => ProposalStatus::Pending(
                match inner
                    .proposal
                    .ok_or(Error::NotFound(String::from("pending proposal")))?
                {
                    ProtoPendingProposalEnum::Spending(inner) => PendingProposal::Spending {
                        descriptor: Descriptor::from_str(&inner.descriptor)?,
                        destination: match inner
                            .destination
                            .ok_or(Error::NotFound(String::from("destination")))?
                            .destination
                            .ok_or(Error::NotFound(String::from("destination")))?
                        {
                            ProtoDestinationEnum::Drain(address) => Destination::Drain(
                                Address::from_str(&address)?.require_network(network)?,
                            ),
                            ProtoDestinationEnum::Single(recipient) => {
                                Destination::Single(Recipient {
                                    address: Address::from_str(&recipient.address)?
                                        .require_network(network)?,
                                    amount: Amount::from_sat(recipient.amount),
                                })
                            }
                            ProtoDestinationEnum::Multiple(ProtoMultipleRecipients {
                                recipients,
                            }) => Destination::Multiple(
                                recipients
                                    .into_iter()
                                    .filter_map(|r| {
                                        Some(Recipient {
                                            address: Address::from_str(&r.address)
                                                .ok()?
                                                .require_network(network)
                                                .ok()?,
                                            amount: Amount::from_sat(r.amount),
                                        })
                                    })
                                    .collect(),
                            ),
                        },
                        psbt: PartiallySignedTransaction::from_str(&inner.psbt)?,
                    },
                    ProtoPendingProposalEnum::ProofOfReserve(inner) => {
                        PendingProposal::ProofOfReserve {
                            descriptor: Descriptor::from_str(&inner.descriptor)?,
                            message: inner.message,
                            psbt: PartiallySignedTransaction::from_str(&inner.psbt)?,
                        }
                    }
                    ProtoPendingProposalEnum::KeyAgentPayment(inner) => {
                        let recipient: ProtoRecipient = inner
                            .recipient
                            .ok_or(Error::NotFound(String::from("recipient")))?;
                        let period: ProtoPeriod = inner
                            .period
                            .ok_or(Error::NotFound(String::from("period")))?;
                        PendingProposal::KeyAgentPayment {
                            descriptor: Descriptor::from_str(&inner.descriptor)?,
                            signer_descriptor: Descriptor::from_str(&inner.signer_descriptor)?,
                            recipient: Recipient {
                                address: Address::from_str(&recipient.address)?
                                    .require_network(network)?,
                                amount: Amount::from_sat(recipient.amount),
                            },
                            period: Period {
                                from: period.from.into(),
                                to: period.to.into(),
                            },
                            psbt: PartiallySignedTransaction::from_str(&inner.psbt)?,
                        }
                    }
                },
            ),
            ProtoProposalStatusEnum::Completed(inner) => {
                ProposalStatus::Completed(inner.try_into()?)
            }
        };

        Ok(Self {
            vault_id,
            status,
            network,
            timestamp: Timestamp::from(value.timestamp),
            description: value.description,
        })
    }
}
