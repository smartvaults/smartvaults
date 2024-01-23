// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Vault v2

use core::cmp::Ordering;
use core::ops::Deref;

use nostr::{Event, EventBuilder, Keys, Tag, Timestamp};
use prost::Message;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::crypto::hash;
use smartvaults_core::policy::Policy;
use smartvaults_core::secp256k1::{SecretKey, XOnlyPublicKey};
use smartvaults_core::PolicyTemplate;

pub mod id;
pub mod metadata;
mod proto;

pub use self::id::VaultIdentifier;
pub use self::metadata::VaultMetadata;
use super::constants::{VAULT_KIND_V2, WRAPPER_EXIPRATION, WRAPPER_KIND};
use super::message::{EncodingVersion, ProtocolEncoding, ProtocolEncryption};
use super::proto::vault::ProtoVault;
use super::{Error, NostrPublicIdentifier, Wrapper};

/// Vault version
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Version {
    /// V1
    #[default]
    V1 = 0x01,
}

/// Vault
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Vault {
    version: Version,
    policy: Policy,
    shared_key: SecretKey,
}

impl PartialOrd for Vault {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Vault {
    fn cmp(&self, other: &Self) -> Ordering {
        other.policy.cmp(&self.policy)
    }
}

impl Deref for Vault {
    type Target = Policy;

    fn deref(&self) -> &Self::Target {
        &self.policy
    }
}

impl Vault {
    /// Construct from descriptor or uncompiled policy
    pub fn new<S>(descriptor: S, network: Network, shared_key: SecretKey) -> Result<Self, Error>
    where
        S: AsRef<str>,
    {
        Ok(Self {
            version: Version::default(),
            policy: Policy::from_desc_or_miniscript(descriptor, network)?,
            shared_key,
        })
    }

    /// Construct from [`PolicyTemplate`]
    pub fn from_template(
        template: PolicyTemplate,
        network: Network,
        shared_key: SecretKey,
    ) -> Result<Self, Error> {
        Ok(Self {
            version: Version::default(),
            policy: Policy::from_template(template, network)?,
            shared_key,
        })
    }

    /// Deterministic identifier
    pub fn id(&self) -> VaultIdentifier {
        VaultIdentifier::from(self.policy.as_descriptor())
    }

    /// Get [`Version`]
    pub fn version(&self) -> Version {
        self.version
    }

    /// Get [`Policy`]
    pub fn policy(&self) -> Policy {
        self.policy.clone()
    }

    /// Get [`SecretKey`]
    pub fn shared_key(&self) -> SecretKey {
        self.shared_key
    }

    /// Generate deterministic Nostr Public Identifier
    pub fn nostr_public_identifier(&self, keys: &Keys) -> NostrPublicIdentifier {
        // TODO: use keys.public_key()? Or secret_key()?
        let unhashed = format!(
            "{}:{}:{}",
            self.policy.as_descriptor(),
            self.shared_key.display_secret(),
            keys.public_key()
        );
        NostrPublicIdentifier::from(hash::sha256(unhashed))
    }
}

impl ProtocolEncoding for Vault {
    type Err = Error;

    fn protocol_network(&self) -> Network {
        self.network()
    }

    fn pre_encoding(&self) -> (EncodingVersion, Vec<u8>) {
        let vault: ProtoVault = self.into();
        (EncodingVersion::ProtoBuf, vault.encode_to_vec())
    }

    fn decode_protobuf(data: &[u8]) -> Result<Self, Self::Err> {
        let vault: ProtoVault = ProtoVault::decode(data)?;
        Self::try_from(vault)
    }
}

impl ProtocolEncryption for Vault {
    type Err = Error;
}

/// Build [`Vault`] invitation [`Event`]
pub fn build_invitation_event(
    vault: &Vault,
    receiver: XOnlyPublicKey,
    sender: Option<XOnlyPublicKey>,
) -> Result<Event, Error> {
    // Compose wrapper
    let wrapper: Wrapper = Wrapper::VaultInvite {
        vault: vault.clone(),
        sender,
    };

    // Encrypt
    let keys = Keys::generate();
    let encrypted_content: String = wrapper.encrypt(&keys.secret_key()?, &receiver)?;

    // Compose and sign event
    Ok(EventBuilder::new(
        WRAPPER_KIND,
        encrypted_content,
        [
            Tag::public_key(receiver),
            Tag::Expiration(Timestamp::now() + WRAPPER_EXIPRATION),
        ],
    )
    .to_event(&keys)?)
}

/// Build [`Vault`] event (used to accept an invitation)
///
/// Must use **own** [`Keys`] (not random or shared key)!
pub fn build_event(keys: &Keys, vault: &Vault) -> Result<Event, Error> {
    // Encrypt
    let encrypted_content: String = vault.encrypt_with_keys(keys)?;

    // Compose and build event
    let identifier: String = vault.nostr_public_identifier(keys).to_string();
    Ok(EventBuilder::new(
        VAULT_KIND_V2,
        encrypted_content,
        [Tag::Identifier(identifier)],
    )
    .to_event(keys)?)
}

#[cfg(bench)]
mod benches {
    use test::{black_box, Bencher};

    use super::*;

    const NETWORK: Network = Network::Testnet;
    const SECRET_KEY: &str = "6b911fd37cdf5c81d4c0adb1ab7fa822ed253ab0ad9aa18d77257c88b29b718e";

    #[bench]
    pub fn encrypt_vault(bh: &mut Bencher) {
        let desc = "tr(c0e6675756101c53287237945c4ed0fbb780b20c5ca6e36b4178ac89075f629c,multi_a(2,[7356e457/86'/1'/784923']tpubDCvLwbJPseNux9EtPbrbA2tgDayzptK4HNkky14Cw6msjHuqyZCE88miedZD86TZUb29Rof3sgtREU4wtzofte7QDSWDiw8ZU6ZYHmAxY9d/0/*,[4eb5d5a1/86'/1'/784923']tpubDCLskGdzStPPo1auRQygJUfbmLMwujWr7fmekdUMD7gqSpwEcRso4CfiP5GkRqfXFYkfqTujyvuehb7inymMhBJFdbJqFyHsHVRuwLKCSe9/0/*))#ccsgt5j5";
        let shared_key = Keys::generate();
        let vault = Vault::new(desc, NETWORK, shared_key.secret_key().unwrap()).unwrap();

        let secret_key = SecretKey::from_str(SECRET_KEY).unwrap();
        let keys = Keys::new(secret_key);

        bh.iter(|| {
            black_box(vault.encrypt_with_keys(&keys)).unwrap();
        });
    }

    #[bench]
    pub fn decrypt_vault(bh: &mut Bencher) {
        let encrypted_vault = "AfJFkHTpOdA7RR6qfam/Pj6p37hqz0h0FtIZqV96LvkMsHeZUFIG7d154QDyQUdelV/C6n4kupJwElqTiJD9JXiLZixlrGHJrswwxAYRjTBqtT5pQAay3f2jwNO6/MeYYA7q0mDh2FpXc/7II9CI0wKoVZWg3aZz+D3F6RkCPwMChlSjq616BlyBxHVQPo2X4PgCQPuGwBUyr+ED999wFQl5i6389BW1n5A+DIimbLPegW4dAeZPqASZWc/mbOgZwif8MN0NQjoy3ExTuGY9cxDRq47eKrJnrvxe/xIgePiWI8FsAVnxf43p9jaRthXpS/bLDyjcXTGTd+Jv8f2/xmANsCIHS0hEy9QZFUml1vsMUyo3hKPxhgubMsmMm0f/HYOdO8H/QYHYvKv9bBnGK8F7fn5oQcIiEA4A5sDc9e9ZJM4BjA+rxypF0boE8PGR68MSkFSMuTwgd3lNnfNeKv6IdtA9RaRKloP1c2f+nREclpXEh4HL31hM+VngWou9zoWSaDpOnwT9r+bnz1zi7/rLsn60CswfK5OSnOvSa+ssr16QSAPyV8zfotV7HR9yvHH8qXtykjqkkM+ImYasT6JUWTyPYrf4EG0=";

        let secret_key = SecretKey::from_str(SECRET_KEY).unwrap();
        let keys = Keys::new(secret_key);

        bh.iter(|| {
            black_box(Vault::decrypt_with_keys(&keys, encrypted_vault)).unwrap();
        });
    }
}
