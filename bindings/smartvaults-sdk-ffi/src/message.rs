// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::sync::Arc;

use nostr_ffi::{EventId, PublicKey};
use smartvaults_sdk::{EventHandled as EventHandledSdk, Message as MessageSdk};
use uniffi::Enum;

#[derive(Enum)]
pub enum EventHandled {
    SharedKey { event_id: Arc<EventId> },
    Policy { policy_id: Arc<EventId> },
    Proposal { proposal_id: Arc<EventId> },
    Approval { proposal_id: Arc<EventId> },
    CompletedProposal { completed_proposal_id: Arc<EventId> },
    Signer { signer_id: Arc<EventId> },
    MySharedSigner { my_shared_signer_id: Arc<EventId> },
    SharedSigner { shared_signer_id: Arc<EventId> },
    Contacts,
    Metadata { public_key: Arc<PublicKey> },
    NostrConnectRequest { request_id: Arc<EventId> },
    Label,
    EventDeletion,
    RelayList,
    KeyAgentSignerOffering,
    VerifiedKeyAgents,
}

impl From<EventHandledSdk> for EventHandled {
    fn from(value: EventHandledSdk) -> Self {
        match value {
            EventHandledSdk::SharedKey(id) => Self::SharedKey {
                event_id: Arc::new(id.into()),
            },
            EventHandledSdk::Policy(id) => Self::Policy {
                policy_id: Arc::new(id.into()),
            },
            EventHandledSdk::Proposal(id) => Self::Proposal {
                proposal_id: Arc::new(id.into()),
            },
            EventHandledSdk::Approval { proposal_id } => Self::Approval {
                proposal_id: Arc::new(proposal_id.into()),
            },
            EventHandledSdk::CompletedProposal(id) => Self::CompletedProposal {
                completed_proposal_id: Arc::new(id.into()),
            },
            EventHandledSdk::Signer(id) => Self::Signer {
                signer_id: Arc::new(id.into()),
            },
            EventHandledSdk::MySharedSigner(id) => Self::MySharedSigner {
                my_shared_signer_id: Arc::new(id.into()),
            },
            EventHandledSdk::SharedSigner(id) => Self::SharedSigner {
                shared_signer_id: Arc::new(id.into()),
            },
            EventHandledSdk::Contacts => Self::Contacts,
            EventHandledSdk::Metadata(pk) => Self::Metadata {
                public_key: Arc::new(pk.into()),
            },
            EventHandledSdk::NostrConnectRequest(id) => Self::NostrConnectRequest {
                request_id: Arc::new(id.into()),
            },
            EventHandledSdk::Label => Self::Label,
            EventHandledSdk::EventDeletion => Self::EventDeletion,
            EventHandledSdk::RelayList => Self::RelayList,
            EventHandledSdk::KeyAgentSignerOffering => Self::KeyAgentSignerOffering,
            EventHandledSdk::VerifiedKeyAgents => Self::VerifiedKeyAgents,
        }
    }
}

#[derive(Enum)]
pub enum Message {
    EventHandledMsg { event: EventHandled },
    WalletSyncCompleted { policy_id: Arc<EventId> },
    BlockHeightUpdated,
}

impl From<MessageSdk> for Message {
    fn from(value: MessageSdk) -> Self {
        match value {
            MessageSdk::EventHandled(event) => Self::EventHandledMsg {
                event: event.into(),
            },
            MessageSdk::WalletSyncCompleted(policy_id) => Self::WalletSyncCompleted {
                policy_id: Arc::new(policy_id.into()),
            },
            MessageSdk::BlockHeightUpdated => Self::BlockHeightUpdated,
        }
    }
}
