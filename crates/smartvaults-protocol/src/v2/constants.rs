// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use nostr::Kind;

/// Wrapper Kind (used for vault invitation, approvals, shared signers, ...)
pub const WRAPPER_KIND: Kind = Kind::Custom(8288);

/// Vault kind
pub const VAULT_KIND_V2: Kind = Kind::Custom(8289);

/// Vault Metadata Kind
pub const VAULT_METADATA_KIND_V2: Kind = Kind::ParameterizedReplaceable(38289);

/// Used both for pending and completed proposals
pub const PROPOSAL_KIND_V2: Kind = Kind::ParameterizedReplaceable(39290);
