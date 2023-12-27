// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Protocol V2

pub mod approval;
pub mod constants;
mod core;
mod error;
mod network;
pub mod proposal;
mod proto;
pub mod signer;
pub mod vault;
pub mod wrapper;

pub use self::approval::{Approval, ApprovalType};
pub use self::core::{ProtocolEncoding, ProtocolEncryption};
pub use self::error::Error;
pub use self::network::NetworkMagic;
pub use self::proposal::{
    CompletedProposal, PendingProposal, Period, Proposal, ProposalStatus, ProposalType,
};
pub use self::signer::{Signer, SignerType};
pub use self::vault::{Vault, VaultIdentifier};
use self::wrapper::Wrapper;
