// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Constants

use core::time::Duration;

use nostr::Kind;

/// Smart Vaults Signer derivation path
pub const SMARTVAULTS_ACCOUNT_INDEX: u32 = 784923;

/// Wrapper Kind (used for vault invitation, approvals, shared signers, ...)
pub const WRAPPER_KIND: Kind = Kind::Custom(8288);

/// Vault kind
pub const VAULT_KIND_V2: Kind = Kind::Custom(8289);

/// Vault Metadata Kind
pub const VAULT_METADATA_KIND_V2: Kind = Kind::ParameterizedReplaceable(38289);

/// Used both for pending and completed proposals
pub const PROPOSAL_KIND_V2: Kind = Kind::ParameterizedReplaceable(39290);

/// Signer kind
pub const SIGNER_KIND_V2: Kind = Kind::ParameterizedReplaceable(39294);

/// Wrapper event expiration
pub const WRAPPER_EXIPRATION: Duration = Duration::from_secs(60 * 60 * 24 * 7); // 7 days
