// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_core::bips::bip48::ScriptType;
use smartvaults_core::Purpose;

use super::{Signer, SignerType};
use crate::v2::proto::signer::{ProtoDescriptor, ProtoPurpose, ProtoSigner, ProtoSignerType};

impl From<&Purpose> for ProtoPurpose {
    fn from(purpose: &Purpose) -> Self {
        match purpose {
            Purpose::BIP44 => Self::Bip44,
            Purpose::BIP48 { script } => match script {
                ScriptType::P2SHWSH => Self::Bip481,
                ScriptType::P2WSH => Self::Bip482,
                ScriptType::P2TR => Self::Bip483,
            },
            Purpose::BIP49 => Self::Bip49,
            Purpose::BIP84 => Self::Bip84,
            Purpose::BIP86 => Self::Bip86,
        }
    }
}

impl From<SignerType> for ProtoSignerType {
    fn from(signer_type: SignerType) -> Self {
        match signer_type {
            SignerType::Seed => Self::Seed,
            SignerType::Hardware => Self::Hardware,
            SignerType::AirGap => Self::Airgap,
        }
    }
}

impl From<&Signer> for ProtoSigner {
    fn from(signer: &Signer) -> Self {
        let signer_type: ProtoSignerType = signer.r#type.into();
        ProtoSigner {
            fingerprint: signer.fingerprint().to_string(),
            descriptors: signer
                .descriptors()
                .iter()
                .map(|(purpose, desc)| {
                    let purpose: ProtoPurpose = purpose.into();
                    ProtoDescriptor {
                        purpose: purpose as i32,
                        descriptor: desc.to_string(),
                    }
                })
                .collect(),
            r#type: signer_type as i32,
            name: signer.name(),
            description: signer.description(),
        }
    }
}
