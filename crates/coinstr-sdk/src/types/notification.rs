// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::fmt;

use coinstr_core::secp256k1::XOnlyPublicKey;
use coinstr_protocol::v1::util::Serde;
use nostr_sdk::EventId;
use serde::{Deserialize, Serialize};

use crate::util;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Notification {
    NewPolicy(EventId),
    NewProposal(EventId),
    NewApproval {
        proposal_id: EventId,
        public_key: XOnlyPublicKey,
    },
    NewCompletedProposal(EventId),
    NewSharedSigner {
        shared_signer_id: EventId,
        owner_public_key: XOnlyPublicKey,
    },
}

impl Serde for Notification {}

impl fmt::Display for Notification {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NewPolicy(id) => {
                write!(f, "New policy: #{}", util::cut_event_id(*id))
            }
            Self::NewProposal(id) => {
                write!(f, "New proposal: #{}", util::cut_event_id(*id))
            }
            Self::NewApproval {
                proposal_id,
                public_key,
            } => {
                write!(
                    f,
                    "{} approved proposal #{}",
                    util::cut_public_key(*public_key),
                    util::cut_event_id(*proposal_id)
                )
            }
            Self::NewCompletedProposal(id) => {
                write!(f, "New finalized proposal: #{}", util::cut_event_id(*id))
            }
            Self::NewSharedSigner {
                shared_signer_id,
                owner_public_key,
            } => {
                write!(
                    f,
                    "{} shared a signer with you: #{}",
                    util::cut_public_key(*owner_public_key),
                    util::cut_event_id(*shared_signer_id)
                )
            }
        }
    }
}
