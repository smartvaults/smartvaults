// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::time::Duration;

use nostr::Kind;

// Kinds
pub const SHARED_KEY_KIND: Kind = Kind::Custom(9288);
pub const POLICY_KIND: Kind = Kind::Custom(9289);
pub const PROPOSAL_KIND: Kind = Kind::Custom(9290);
pub const APPROVED_PROPOSAL_KIND: Kind = Kind::Custom(9291);
pub const COMPLETED_PROPOSAL_KIND: Kind = Kind::Custom(9292);
pub const SIGNERS_KIND: Kind = Kind::Custom(9294);
pub const SHARED_SIGNERS_KIND: Kind = Kind::Custom(9295);
pub const LABELS_KIND: Kind = Kind::Custom(32121);

// Expirations
pub const APPROVED_PROPOSAL_EXPIRATION: Duration = Duration::from_secs(60 * 60 * 24 * 7);
