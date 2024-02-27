// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

include!(concat!(env!("OUT_DIR"), "/proposal.rs"));

pub use self::destination::{
    Destination as ProtoDestinationEnum, MultipleRecipients as ProtoMultipleRecipients,
};
pub use self::proposal_status::completed_proposal::{
    KeyAgentPayment as ProtoCompletedKeyAgentPayment,
    ProofOfReserve as ProtoCompletedProofOfReserve, Proposal as ProtoCompletedProposalEnum,
    Spending as ProtoCompletedSpending,
};
pub use self::proposal_status::pending_proposal::{
    KeyAgentPayment as ProtoPendingKeyAgentPayment, ProofOfReserve as ProtoPendingProofOfReserve,
    Proposal as ProtoPendingProposalEnum, Spending as ProtoPendingSpending,
};
pub use self::proposal_status::{
    CompletedProposal as ProtoCompletedProposal, PendingProposal as ProtoPendingProposal,
    Proposal as ProtoProposalStatusEnum,
};
pub use self::{
    Destination as ProtoDestination, Period as ProtoPeriod, Proposal as ProtoProposal,
    ProposalIdentifier as ProtoProposalIdentifier, ProposalStatus as ProtoProposalStatus,
    Recipient as ProtoRecipient,
};
