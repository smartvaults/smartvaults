// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::str::FromStr;
use std::time::Duration;

use nostr::Kind;
use once_cell::sync::Lazy;
use smartvaults_core::secp256k1::XOnlyPublicKey;

// Smart Vaults Public Keys
pub static SMARTVAULTS_MAINNET_PUBLIC_KEY: Lazy<XOnlyPublicKey> = Lazy::new(|| {
    XOnlyPublicKey::from_str("5f5d73eee1b08e1743142538b1acb65ec16c1475b6b6902ca2380b19b6b4c006")
        .expect("Invalid public key") // TODO: to replace
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
pub const LABELS_KIND: Kind = Kind::Custom(32121);
pub const KEY_AGENT_SIGNER_OFFERING_KIND: Kind = Kind::Custom(32122);
pub const KEY_AGENT_VERIFIED: Kind = Kind::Custom(32123);

// Expirations
pub const APPROVED_PROPOSAL_EXPIRATION: Duration = Duration::from_secs(60 * 60 * 24 * 7);
