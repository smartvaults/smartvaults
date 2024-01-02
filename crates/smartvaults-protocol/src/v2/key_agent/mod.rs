// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Key Agent

use nostr::{Event, EventBuilder, Keys, Tag};

use super::Signer;
use crate::v1::constants::KEY_AGENT_SIGNER_OFFERING_KIND;
pub use crate::v1::key_agent::*;
use crate::v1::Serde;
use crate::v2::Error;

/// Build signer offering event for [`Signer`]
pub fn build_event(
    keys: &Keys,
    signer: &Signer,
    offering: &SignerOffering,
) -> Result<Event, Error> {
    let content: String = offering.as_json();
    Ok(EventBuilder::new(
        KEY_AGENT_SIGNER_OFFERING_KIND,
        content,
        [Tag::Identifier(
            signer.nostr_public_identifier().to_string(),
        )],
    )
    .to_event(keys)?)
}
