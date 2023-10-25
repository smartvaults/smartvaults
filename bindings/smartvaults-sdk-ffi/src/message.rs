// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use smartvaults_sdk::{EventHandled as EventHandledSdk, Message as MessageSdk};

pub enum EventHandled {
    EHSharedKey { event_id: String },
    EHPolicy { policy_id: String },
    EHProposal { proposal_id: String },
    EHApproval { proposal_id: String },
    EHCompletedProposal { completed_proposal_id: String },
    EHSigner { signer_id: String },
    EHMySharedSigner { my_shared_signer_id: String },
    EHSharedSigner { shared_signer_id: String },
    EHContacts(),
    EHMetadata { public_key: String },
    EHNostrConnectRequest { request_id: String },
    EHLabel(),
    EHEventDeletion(),
    EHRelayList(),
    EHKeyAgentSignerOffering(),
}

impl From<EventHandledSdk> for EventHandled {
    fn from(value: EventHandledSdk) -> Self {
        match value {
            EventHandledSdk::SharedKey(id) => Self::EHSharedKey {
                event_id: id.to_hex(),
            },
            EventHandledSdk::Policy(id) => Self::EHPolicy {
                policy_id: id.to_hex(),
            },
            EventHandledSdk::Proposal(id) => Self::EHProposal {
                proposal_id: id.to_hex(),
            },
            EventHandledSdk::Approval { proposal_id } => Self::EHApproval {
                proposal_id: proposal_id.to_hex(),
            },
            EventHandledSdk::CompletedProposal(id) => Self::EHCompletedProposal {
                completed_proposal_id: id.to_hex(),
            },
            EventHandledSdk::Signer(id) => Self::EHSigner {
                signer_id: id.to_hex(),
            },
            EventHandledSdk::MySharedSigner(id) => Self::EHMySharedSigner {
                my_shared_signer_id: id.to_hex(),
            },
            EventHandledSdk::SharedSigner(id) => Self::EHSharedSigner {
                shared_signer_id: id.to_hex(),
            },
            EventHandledSdk::Contacts => Self::EHContacts(),
            EventHandledSdk::Metadata(pk) => Self::EHMetadata {
                public_key: pk.to_string(),
            },
            EventHandledSdk::NostrConnectRequest(id) => Self::EHNostrConnectRequest {
                request_id: id.to_hex(),
            },
            EventHandledSdk::Label => Self::EHLabel(),
            EventHandledSdk::EventDeletion => Self::EHEventDeletion(),
            EventHandledSdk::RelayList => Self::EHRelayList(),
            EventHandledSdk::KeyAgentSignerOffering => Self::EHKeyAgentSignerOffering(),
        }
    }
}

pub enum Message {
    EvH { event: EventHandled },
    WalletSyncCompleted { policy_id: String },
    BlockHeightUpdated(),
}

impl From<MessageSdk> for Message {
    fn from(value: MessageSdk) -> Self {
        match value {
            MessageSdk::EventHandled(event) => Self::EvH {
                event: event.into(),
            },
            MessageSdk::WalletSyncCompleted(policy_id) => Self::WalletSyncCompleted {
                policy_id: policy_id.to_hex(),
            },
            MessageSdk::BlockHeightUpdated => Self::BlockHeightUpdated(),
        }
    }
}
