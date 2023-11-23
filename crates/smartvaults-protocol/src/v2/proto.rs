// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

#![allow(clippy::module_inception)]

pub mod vault {
    include!(concat!(env!("OUT_DIR"), "/vault.rs"));

    pub use self::vault::Object as ProtoVaultObject;
    pub use self::{Vault as ProtoVault, VaultV1 as ProtoVaultV1};
}

pub mod proposal {
    mod inner {
        include!(concat!(env!("OUT_DIR"), "/proposal.rs"));
    }

    pub use self::inner::proposal_status::completed_proposal::{
        KeyAgentPayment as ProtoCompletedKeyAgentPayment,
        ProofOfReserve as ProtoCompletedProofOfReserve, Proposal as ProtoCompletedProposalEnum,
        Spending as ProtoCompletedSpending,
    };
    pub use self::inner::proposal_status::pending_proposal::{
        KeyAgentPayment as ProtoPendingKeyAgentPayment,
        ProofOfReserve as ProtoPendingProofOfReserve, Proposal as ProtoPendingProposalEnum,
        Spending as ProtoPendingSpending,
    };
    pub use self::inner::proposal_status::{
        CompletedProposal as ProtoCompletedProposal, PendingProposal as ProtoPendingProposal,
        Proposal as ProtoProposalStatusEnum,
    };
    pub use self::inner::{
        Period as ProtoPeriod, Proposal as ProtoProposal, ProposalStatus as ProtoProposalStatus,
        Recipient as ProtoRecipient,
    };
}

pub mod signer {
    mod inner {
        include!(concat!(env!("OUT_DIR"), "/signer.rs"));
    }

    pub use self::inner::{
        DescriptorKeyValue as ProtoDescriptor, Purpose as ProtoPurpose, Signer as ProtoSigner,
        SignerType as ProtoSignerType,
    };
}

pub mod wrapper {
    include!(concat!(env!("OUT_DIR"), "/wrapper.rs"));

    pub use self::wrapper::Object as ProtoWrapperObject;
    pub use self::Wrapper as ProtoWrapper;
}
