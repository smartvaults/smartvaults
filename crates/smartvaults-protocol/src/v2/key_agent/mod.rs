// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

//! Key Agent

use nostr::{Event, EventBuilder, Tag};
use nostr_signer::NostrSigner;
use smartvaults_core::bitcoin::Network;

use super::Signer;
use crate::v1::constants::{KEY_AGENT_SIGNALING, KEY_AGENT_SIGNER_OFFERING_KIND};
pub use crate::v1::key_agent::*;
use crate::v1::Serde;
use crate::v2::Error;

/// Build key agent signaling event
pub async fn build_key_agent_signaling_event(
    signer: &NostrSigner,
    network: Network,
) -> Result<Event, Error> {
    let identifier: String = network.magic().to_string();
    let builder = EventBuilder::new(KEY_AGENT_SIGNALING, "", [Tag::Identifier(identifier)]);
    Ok(signer.sign_event_builder(builder).await?)
}

/// Build signer offering event for [`Signer`]
pub async fn build_event(
    nostr_signer: &NostrSigner,
    signer: &Signer,
    offering: &SignerOffering,
) -> Result<Event, Error> {
    let content: String = offering.as_json();
    let builder = EventBuilder::new(
        KEY_AGENT_SIGNER_OFFERING_KIND,
        content,
        [Tag::Identifier(
            signer.nostr_public_identifier().to_string(),
        )],
    );

    Ok(nostr_signer.sign_event_builder(builder).await?)
}
