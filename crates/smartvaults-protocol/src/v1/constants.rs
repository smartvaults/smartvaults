// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;
use std::time::Duration;

use nostr::Kind;
use once_cell::sync::Lazy;
use smartvaults_core::secp256k1::XOnlyPublicKey;

// Smart Vaults Public Keys
pub static SMARTVAULTS_MAINNET_PUBLIC_KEY: Lazy<XOnlyPublicKey> = Lazy::new(|| {
    XOnlyPublicKey::from_str("32c961f39afcff6df6abed251b346550329b2dbcabca0667530f0be5054fe7ae")
        .expect("Invalid public key") // npub1xtykruu6lnlkma4ta5j3kdr92qefktdu409qve6npu972p20u7hqhh99km
});
pub static SMARTVAULTS_TESTNET_PUBLIC_KEY: Lazy<XOnlyPublicKey> = Lazy::new(|| {
    XOnlyPublicKey::from_str("2c2dcda12330dda3b9600237a419003c5d9bf3d757303e63ecee121b4aaa2fa0")
        .expect("Invalid public key")
});

// Kinds
pub const SHARED_KEY_KIND: Kind = Kind::Custom(9288);
pub const POLICY_KIND: Kind = Kind::Custom(9289);
pub const PROPOSAL_KIND: Kind = Kind::Custom(9290);
pub const APPROVED_PROPOSAL_KIND: Kind = Kind::Custom(9291);
pub const COMPLETED_PROPOSAL_KIND: Kind = Kind::Custom(9292);
pub const SIGNERS_KIND: Kind = Kind::Custom(9294);
pub const SHARED_SIGNERS_KIND: Kind = Kind::Custom(9295);
pub const LABELS_KIND: Kind = Kind::ParameterizedReplaceable(32121);
pub const KEY_AGENT_SIGNER_OFFERING_KIND: Kind = Kind::ParameterizedReplaceable(32122);
pub const KEY_AGENT_VERIFIED: Kind = Kind::ParameterizedReplaceable(32123);
pub const KEY_AGENT_SIGNALING: Kind = Kind::ParameterizedReplaceable(32124);

// Expirations
pub const APPROVED_PROPOSAL_EXPIRATION: Duration = Duration::from_secs(60 * 60 * 24 * 7);
