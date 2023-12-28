// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;

use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;

use super::Approval;
use crate::v2::approval::Version;
use crate::v2::proto::approval::{
    ProtoApproval, ProtoApprovalObject, ProtoApprovalType, ProtoApprovalV1,
};
use crate::v2::proto::vault::ProtoVaultIdentifier;
use crate::v2::{ApprovalType, Error, NetworkMagic, VaultIdentifier};

impl From<&Approval> for ProtoApproval {
    fn from(approval: &Approval) -> Self {
        let approval_type: ProtoApprovalType = approval.r#type.into();
        Self {
            object: Some(ProtoApprovalObject::V1(ProtoApprovalV1 {
                vault_id: Some(approval.vault_id.into()),
                r#type: approval_type as i32,
                psbt: approval.psbt.to_string(),
                network: approval.network.magic().to_bytes().to_vec(),
            })),
        }
    }
}

impl From<ApprovalType> for ProtoApprovalType {
    fn from(value: ApprovalType) -> Self {
        match value {
            ApprovalType::Spending => Self::Spending,
            ApprovalType::ProofOfReserve => Self::ProofOfReserve,
            ApprovalType::KeyAgentPayment => Self::KeyAgentPayment,
        }
    }
}

impl From<ProtoApprovalType> for ApprovalType {
    fn from(value: ProtoApprovalType) -> Self {
        match value {
            ProtoApprovalType::Spending => Self::Spending,
            ProtoApprovalType::ProofOfReserve => Self::ProofOfReserve,
            ProtoApprovalType::KeyAgentPayment => Self::KeyAgentPayment,
        }
    }
}

impl TryFrom<ProtoApproval> for Approval {
    type Error = Error;
    fn try_from(value: ProtoApproval) -> Result<Self, Self::Error> {
        let approval = value
            .object
            .ok_or(Error::NotFound(String::from("approval object")))?;
        match approval {
            ProtoApprovalObject::V1(v1) => {
                let vault_id: ProtoVaultIdentifier = v1
                    .vault_id
                    .ok_or(Error::NotFound(String::from("vault identifier")))?;

                Ok(Self {
                    vault_id: VaultIdentifier::from_slice(&vault_id.id)?,
                    version: Version::V1,
                    psbt: PartiallySignedTransaction::from_str(&v1.psbt)?,
                    r#type: ProtoApprovalType::try_from(v1.r#type)?.into(),
                    network: NetworkMagic::from_slice(&v1.network)?.into(),
                })
            }
        }
    }
}
