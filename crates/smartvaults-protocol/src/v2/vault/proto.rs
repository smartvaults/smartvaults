// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use core::str::FromStr;

use smartvaults_core::hashes::Hash;
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::secp256k1::{SecretKey, XOnlyPublicKey};
use smartvaults_core::Policy;

use super::{Vault, VaultIdentifier, VaultInvite, VaultMetadata, Version};
use crate::v2::proto::vault::{
    ProtoVault, ProtoVaultIdentifier, ProtoVaultInvite, ProtoVaultMetadata, ProtoVaultObject,
    ProtoVaultV1,
};
use crate::v2::{Error, NetworkMagic};

impl From<&VaultIdentifier> for ProtoVaultIdentifier {
    fn from(id: &VaultIdentifier) -> Self {
        Self {
            id: id.as_byte_array().to_vec(),
        }
    }
}

impl From<VaultIdentifier> for ProtoVaultIdentifier {
    fn from(id: VaultIdentifier) -> Self {
        Self {
            id: id.to_byte_array().to_vec(),
        }
    }
}

impl From<&Vault> for ProtoVault {
    fn from(vault: &Vault) -> Self {
        Self {
            object: Some(ProtoVaultObject::V1(ProtoVaultV1 {
                descriptor: vault.as_descriptor().to_string(),
                network: vault.network().magic().to_bytes().to_vec(),
                shared_key: vault.shared_key.secret_bytes().to_vec(),
            })),
        }
    }
}

impl TryFrom<ProtoVault> for Vault {
    type Error = Error;

    fn try_from(vault: ProtoVault) -> Result<Self, Self::Error> {
        match vault.object {
            Some(obj) => match obj {
                ProtoVaultObject::V1(v) => {
                    let descriptor: Descriptor<String> = Descriptor::from_str(&v.descriptor)?;
                    let network: NetworkMagic = NetworkMagic::from_slice(&v.network)?;
                    let shared_key: SecretKey = SecretKey::from_slice(&v.shared_key)?;

                    Ok(Self {
                        version: Version::V1,
                        policy: Policy::new(descriptor, *network)?,
                        shared_key,
                    })
                }
            },
            None => Err(Error::NotFound(String::from("protobuf vault obj"))),
        }
    }
}

impl From<&VaultMetadata> for ProtoVaultMetadata {
    fn from(metadata: &VaultMetadata) -> Self {
        Self {
            vault_id: Some(metadata.vault_id().into()),
            name: metadata.name.clone(),
            description: metadata.description.clone(),
        }
    }
}

impl TryFrom<ProtoVaultMetadata> for VaultMetadata {
    type Error = Error;

    fn try_from(metadata: ProtoVaultMetadata) -> Result<Self, Self::Error> {
        let vault_id: ProtoVaultIdentifier = metadata
            .vault_id
            .ok_or(Error::NotFound(String::from("vault identifier")))?;
        let vault_id: VaultIdentifier = VaultIdentifier::from_slice(&vault_id.id)?;
        let mut m = VaultMetadata::new(vault_id);
        m.change_name(metadata.name);
        m.change_description(metadata.description);
        Ok(m)
    }
}

impl From<&VaultInvite> for ProtoVaultInvite {
    fn from(invite: &VaultInvite) -> Self {
        Self {
            vault: Some(invite.vault().into()),
            sender: invite.sender().map(|p| p.to_string()),
            message: invite.message().to_string(),
        }
    }
}

impl TryFrom<ProtoVaultInvite> for VaultInvite {
    type Error = Error;

    fn try_from(invite: ProtoVaultInvite) -> Result<Self, Self::Error> {
        let vault: ProtoVault = invite.vault.ok_or(Error::NotFound(String::from("vault")))?;
        let vault: Vault = Vault::try_from(vault)?;
        let sender: Option<XOnlyPublicKey> = match invite.sender {
            Some(public_key) => Some(XOnlyPublicKey::from_str(&public_key)?),
            None => None,
        };
        Ok(Self::new(vault, sender, invite.message))
    }
}
