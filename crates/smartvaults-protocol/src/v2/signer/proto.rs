// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::BTreeMap;
use std::str::FromStr;

use smartvaults_core::bips::bip32::Fingerprint;
use smartvaults_core::bips::bip48::ScriptType;
use smartvaults_core::miniscript::DescriptorPublicKey;
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::{CoreSigner, Purpose};

use super::{SharedSigner, Signer, SignerType};
use crate::v2::proto::signer::{
    ProtoDescriptor, ProtoPurpose, ProtoSharedSigner, ProtoSigner, ProtoSignerType,
};
use crate::v2::{Error, NetworkMagic};

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

impl From<ProtoPurpose> for Purpose {
    fn from(purpose: ProtoPurpose) -> Self {
        match purpose {
            ProtoPurpose::Bip44 => Self::BIP44,
            ProtoPurpose::Bip481 => Self::BIP48 {
                script: ScriptType::P2SHWSH,
            },
            ProtoPurpose::Bip482 => Self::BIP48 {
                script: ScriptType::P2WSH,
            },
            ProtoPurpose::Bip483 => Self::BIP48 {
                script: ScriptType::P2TR,
            },
            ProtoPurpose::Bip49 => Self::BIP49,
            ProtoPurpose::Bip84 => Self::BIP84,
            ProtoPurpose::Bip86 => Self::BIP86,
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

impl From<ProtoSignerType> for SignerType {
    fn from(value: ProtoSignerType) -> Self {
        match value {
            ProtoSignerType::Seed => Self::Seed,
            ProtoSignerType::Hardware => Self::Hardware,
            ProtoSignerType::Airgap => Self::AirGap,
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
            network: signer.network().magic().to_bytes().to_vec(),
            r#type: signer_type as i32,
            name: signer.name(),
            description: signer.description(),
        }
    }
}

impl TryFrom<ProtoSigner> for Signer {
    type Error = Error;

    fn try_from(value: ProtoSigner) -> Result<Self, Self::Error> {
        let proto_signer_type: ProtoSignerType = ProtoSignerType::try_from(value.r#type)?;
        let fingerprint: Fingerprint = Fingerprint::from_str(&value.fingerprint)?;
        let network: NetworkMagic = NetworkMagic::from_slice(&value.network)?;

        let mut descriptors: BTreeMap<Purpose, DescriptorPublicKey> = BTreeMap::new();

        for ProtoDescriptor {
            purpose,
            descriptor,
        } in value.descriptors.into_iter()
        {
            let purpose: ProtoPurpose = ProtoPurpose::try_from(purpose)?;
            let purpose: Purpose = Purpose::from(purpose);
            let descriptor: DescriptorPublicKey = DescriptorPublicKey::from_str(&descriptor)?;
            descriptors.insert(purpose, descriptor);
        }

        Ok(Self {
            name: value.name,
            description: value.description,
            core: CoreSigner::new(fingerprint, descriptors, *network)?,
            r#type: SignerType::from(proto_signer_type),
        })
    }
}

impl From<&SharedSigner> for ProtoSharedSigner {
    fn from(signer: &SharedSigner) -> Self {
        ProtoSharedSigner {
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
            network: signer.network().magic().to_bytes().to_vec(),
            owner: signer.owner().to_string(),
            receiver: signer.receiver().to_string(),
        }
    }
}

impl TryFrom<ProtoSharedSigner> for SharedSigner {
    type Error = Error;

    fn try_from(value: ProtoSharedSigner) -> Result<Self, Self::Error> {
        let fingerprint: Fingerprint = Fingerprint::from_str(&value.fingerprint)?;
        let network: NetworkMagic = NetworkMagic::from_slice(&value.network)?;

        let mut descriptors: BTreeMap<Purpose, DescriptorPublicKey> = BTreeMap::new();

        for ProtoDescriptor {
            purpose,
            descriptor,
        } in value.descriptors.into_iter()
        {
            let purpose: ProtoPurpose = ProtoPurpose::try_from(purpose)?;
            let purpose: Purpose = Purpose::from(purpose);
            let descriptor: DescriptorPublicKey = DescriptorPublicKey::from_str(&descriptor)?;
            descriptors.insert(purpose, descriptor);
        }

        Ok(Self::new(
            XOnlyPublicKey::from_str(&value.owner)?,
            XOnlyPublicKey::from_str(&value.receiver)?,
            CoreSigner::new(fingerprint, descriptors, *network)?,
        ))
    }
}
