// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

//! Smart Vaults Protocol V1

#![allow(missing_docs)]

pub mod builder;
pub mod constants;
pub mod key_agent;
pub mod label;
mod network;
pub mod signer;
pub mod util;
pub mod vault;

pub use self::builder::{Error as SmartVaultsEventBuilderError, SmartVaultsEventBuilder};
pub use self::key_agent::{
    BasisPoints, DeviceType, KeyAgentMetadata, Price, SignerOffering, Temperature,
    VerifiedKeyAgentData, VerifiedKeyAgents,
};
pub use self::label::{Label, LabelData, LabelKind};
pub use self::signer::{SharedSigner, Signer, SignerType};
pub use self::util::{Encryption, EncryptionError, Serde, SerdeSer};
pub use self::vault::Vault;
