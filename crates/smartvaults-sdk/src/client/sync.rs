// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashSet;
use std::ops::Add;
use std::str::FromStr;
use std::sync::atomic::Ordering;
use std::time::Duration;

use async_utility::thread;
use futures_util::stream::AbortHandle;
use nostr_sdk::database::NostrDatabaseExt;
use nostr_sdk::nips::nip46::{Message as NIP46Message, Request as NIP46Request};
use nostr_sdk::nips::{nip04, nip65};
use nostr_sdk::{
    ClientMessage, Event, EventBuilder, EventId, Filter, JsonUtil, Keys, Kind, NegentropyOptions,
    RelayMessage, RelayPoolNotification, Result, Tag, TagKind, Timestamp, Url,
};
use smartvaults_core::bdk::chain::ConfirmationTime;
use smartvaults_core::bitcoin::secp256k1::{SecretKey, XOnlyPublicKey};
use smartvaults_core::bitcoin::Network;
use smartvaults_core::{
    ApprovedProposal, CompletedProposal, Policy, Proposal, SharedSigner, Signer,
};
use smartvaults_protocol::v1::constants::{
    APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, KEY_AGENT_SIGNER_OFFERING_KIND,
    KEY_AGENT_VERIFIED, LABELS_KIND, POLICY_KIND, PROPOSAL_KIND, SHARED_KEY_KIND,
    SHARED_SIGNERS_KIND, SIGNERS_KIND, SMARTVAULTS_MAINNET_PUBLIC_KEY,
    SMARTVAULTS_TESTNET_PUBLIC_KEY,
};
use smartvaults_protocol::v1::{Encryption, Label, Serde, VerifiedKeyAgents};
use smartvaults_sdk_sqlite::model::InternalGetPolicy;
use smartvaults_sdk_sqlite::Type;
use tokio::sync::broadcast::Receiver;

use super::{Error, SmartVaults};
use crate::constants::WALLET_SYNC_INTERVAL;

use crate::manager::{Error as ManagerError, WalletError};
use crate::util;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventHandled {
    SharedKey(EventId),
    Policy(EventId),
    Proposal(EventId),
    Approval { proposal_id: EventId },
    CompletedProposal(EventId),
    Signer(EventId),
    MySharedSigner(EventId),
    SharedSigner(EventId),
    Contacts,
    Metadata(XOnlyPublicKey),
    NostrConnectRequest(EventId),
    Label,
    EventDeletion,
    RelayList,
    KeyAgentSignerOffering,
}

#[derive(Debug, Clone, Copy)]
pub enum Message {
    EventHandled(EventHandled),
    WalletSyncCompleted(EventId),
    BlockHeightUpdated,
}

impl SmartVaults {
    fn block_height_syncer(&self) -> AbortHandle {
        let this = self.clone();
        thread::abortable(async move {
            loop {
                match this.config.electrum_endpoint().await {
                    Ok(endpoint) => {
                        let proxy = this.config.proxy().await.ok();
                        match this.manager.sync_block_height(endpoint, proxy).await {
                            Ok(_) => {
                                let _ = this.sync_channel.send(Message::BlockHeightUpdated);
                            }
                            Err(e) => tracing::error!("Impossible to sync block height: {e}"),
                        }
                    }
                    Err(e) => tracing::error!("Impossible to sync wallets: {e}"),
                }

                thread::sleep(Duration::from_secs(10)).await;
            }
        })
    }

    fn policies_syncer(&self) -> AbortHandle {
        let this = self.clone();
        thread::abortable(async move {
            loop {
                match this.config.electrum_endpoint().await {
                    Ok(endpoint) => match this.db.get_policies().await {
                        Ok(policies) => {
                            let proxy = this.config.proxy().await.ok();
                            for InternalGetPolicy {
                                policy_id,
                                last_sync,
                                ..
                            } in policies.into_iter()
                            {
                                let last_sync: Timestamp =
                                    last_sync.unwrap_or_else(|| Timestamp::from(0));
                                if last_sync.add(WALLET_SYNC_INTERVAL) <= Timestamp::now() {
                                    let manager = this.manager.clone();
                                    let db = this.db.clone();
                                    let sync_channel = this.sync_channel.clone();
                                    let endpoint = endpoint.clone();
                                    thread::spawn(async move {
                                        tracing::debug!("Syncing policy {policy_id}");
                                        match manager.sync(policy_id, endpoint, proxy).await {
                                            Ok(_) => {
                                                tracing::info!("Policy {policy_id} synced");
                                                if let Err(e) = db
                                                    .update_last_sync(
                                                        policy_id,
                                                        Some(Timestamp::now()),
                                                    )
                                                    .await
                                                {
                                                    tracing::error!(
                                                        "Impossible to save last policy sync: {e}"
                                                    );
                                                }
                                                let _ = sync_channel
                                                    .send(Message::WalletSyncCompleted(policy_id));
                                            }
                                            Err(ManagerError::Wallet(
                                                WalletError::AlreadySyncing,
                                            )) => tracing::warn!(
                                                "Policy {policy_id} is already syncing"
                                            ),
                                            Err(e) => tracing::error!(
                                                "Impossible to sync policy {policy_id}: {e}"
                                            ),
                                        }
                                    });
                                }
                            }
                        }
                        Err(e) => tracing::error!("Impossible to get policies: {e}"),
                    },
                    Err(e) => tracing::error!("Impossible to sync wallets: {e}"),
                }

                thread::sleep(Duration::from_secs(10)).await;
            }
        })
    }

    fn handle_pending_events(&self) -> AbortHandle {
        let this = self.clone();
        thread::abortable(async move {
            loop {
                match this.db.get_pending_events().await {
                    Ok(events) => {
                        for event in events.into_iter() {
                            let event_id = event.id;
                            if let Err(e) = this.handle_event(event).await {
                                tracing::error!(
                                    "Impossible to handle pending event {event_id}: {e}"
                                );
                            }
                        }
                    }
                    Err(e) => tracing::error!("Impossible to get pending events: {e}"),
                }
                thread::sleep(Duration::from_secs(30)).await;
            }
        })
    }

    fn rebroadcaster(&self) -> AbortHandle {
        let this = self.clone();
        thread::abortable(async move {
            loop {
                // TODO: check last rebroadcast timestamp from db
                if false {
                    match this.rebroadcast_all_events().await {
                        Ok(_) => tracing::info!("All events rebroadcasted to relays"),
                        Err(e) => {
                            tracing::error!("Impossible to rebroadcast events to relays: {e}")
                        }
                    }
                }
                thread::sleep(Duration::from_secs(60)).await;
            }
        })
    }

    pub fn sync_notifications(&self) -> Receiver<Message> {
        self.sync_channel.subscribe()
    }

    pub(crate) async fn sync_filters(&self, since: Timestamp) -> Vec<Filter> {
        let base_filter = Filter::new().kinds(vec![
            POLICY_KIND,
            PROPOSAL_KIND,
            APPROVED_PROPOSAL_KIND,
            COMPLETED_PROPOSAL_KIND,
            SHARED_KEY_KIND,
            SIGNERS_KIND,
            SHARED_SIGNERS_KIND,
            LABELS_KIND,
            Kind::EventDeletion,
        ]);

        let keys: Keys = self.keys().await;
        let public_key: XOnlyPublicKey = keys.public_key();
        let contacts: Vec<XOnlyPublicKey> = self
            .client
            .database()
            .contacts_public_keys(public_key)
            .await
            .unwrap_or_default();

        let author_filter: Filter = base_filter.clone().author(public_key).since(since);
        let pubkey_filter: Filter = base_filter.pubkey(public_key).since(since);
        let nostr_connect_filter = Filter::new()
            .pubkey(public_key)
            .kind(Kind::NostrConnect)
            .since(since);
        let other_filters: Filter = Filter::new()
            .author(public_key)
            .kinds(vec![Kind::Metadata, Kind::ContactList, Kind::RelayList])
            .since(since);
        let key_agents: Filter = Filter::new()
            .kind(KEY_AGENT_SIGNER_OFFERING_KIND)
            .since(since);
        let smartvaults: Filter = Filter::new()
            .author(match self.network {
                Network::Bitcoin => *SMARTVAULTS_MAINNET_PUBLIC_KEY,
                _ => *SMARTVAULTS_TESTNET_PUBLIC_KEY,
            })
            .kinds(vec![KEY_AGENT_VERIFIED]);

        let mut filters = vec![
            author_filter,
            pubkey_filter,
            nostr_connect_filter,
            other_filters,
            key_agents,
            smartvaults,
        ];

        if !contacts.is_empty() {
            filters.push(Filter::new().authors(contacts).since(since));
        }

        filters
    }

    pub(crate) fn sync(&self) {
        if self.syncing.load(Ordering::SeqCst) {
            tracing::warn!("Syncing threads are already running");
        } else {
            let _ = self
                .syncing
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(true));
            let this = self.clone();
            thread::spawn(async move {
                // Sync timechain
                let block_height_syncer: AbortHandle = this.block_height_syncer();
                let policies_syncer: AbortHandle = this.policies_syncer();

                // Pending events handler
                let pending_event_handler = this.handle_pending_events();
                // TODO: let metadata_sync = this.sync_metadata();

                // Rebroadcaster
                let rebroadcaster = this.rebroadcaster();

                for (relay_url, relay) in this.client.relays().await {
                    let last_sync: Timestamp =
                        match this.db.get_last_relay_sync(relay_url.clone()).await {
                            Ok(ts) => ts,
                            Err(e) => {
                                tracing::error!("Impossible to get last relay sync: {e}");
                                Timestamp::from(0)
                            }
                        };
                    let filters: Vec<Filter> = this.sync_filters(last_sync).await;
                    if let Err(e) = relay.subscribe(filters, None).await {
                        tracing::error!("Impossible to subscribe to {relay_url}: {e}");
                    }
                }

                let _ = this
                    .client
                    .handle_notifications(|notification| async {
                        match notification {
                            RelayPoolNotification::Event(_, event) => {
                                let event_id = event.id;
                                if event.is_expired() {
                                    tracing::warn!("Event {event_id} expired");
                                } else if let Err(e) = this.handle_event(event).await {
                                    tracing::error!("Impossible to handle event {event_id}: {e}");
                                }
                            }
                            RelayPoolNotification::Message(relay_url, relay_msg) => {
                                if let RelayMessage::EndOfStoredEvents(subscription_id) = relay_msg {
                                    tracing::debug!("Received new EOSE for {relay_url} with subid {subscription_id}");
                                    if let Ok(relay) = this.client.relay(&relay_url).await {
                                        for (_, subscription) in relay.subscriptions().await.into_iter() {
                                            if subscription.id() == subscription_id {
                                                if let Err(e) = this
                                                    .db
                                                    .save_last_relay_sync(relay_url, Timestamp::now()).await
                                                {
                                                    tracing::error!("Impossible to save last relay sync: {e}");
                                                }
                                                break;
                                            }
                                        }
                                    }
                                }
                            }
                            RelayPoolNotification::RelayStatus { .. } => (),
                            RelayPoolNotification::Stop | RelayPoolNotification::Shutdown => {
                                tracing::debug!("Received stop/shutdown msg");
                                block_height_syncer.abort();
                                policies_syncer.abort();
                                pending_event_handler.abort();
                                rebroadcaster.abort();
                                let _ = this.syncing.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(false));
                            }
                        }

                        Ok(false)
                    })
                    .await;
                tracing::debug!("Exited from nostr sync thread");
            });

            // Negentropy reconciliation
            let this = self.clone();
            thread::spawn(async move {
                let opts = NegentropyOptions::new()
                    .timeout(Duration::from_secs(60))
                    .single_reconciliation_timeout(Duration::from_secs(45));
                for filter in this.sync_filters(Timestamp::from(0)).await.into_iter() {
                    this.client.reconcile(filter, opts).await.unwrap();
                }
            });
        }
    }

    async fn handle_event(&self, event: Event) -> Result<()> {
        if event.kind == SHARED_KEY_KIND {
            let policy_id = util::extract_first_event_id(&event).ok_or(Error::PolicyNotFound)?;
            if !self.db.exists(Type::SharedKey { policy_id }).await? {
                let keys: Keys = self.keys().await;
                let content = nip04::decrypt(&keys.secret_key()?, &event.pubkey, &event.content)?;
                let sk = SecretKey::from_str(&content)?;
                let shared_key = Keys::new(sk);
                self.db.save_shared_key(policy_id, shared_key).await?;
                self.sync_channel
                    .send(Message::EventHandled(EventHandled::SharedKey(event.id)))?;
            }
        } else if event.kind == POLICY_KIND
            && !self
                .db
                .exists(Type::Policy {
                    policy_id: event.id,
                })
                .await?
        {
            if let Ok(shared_key) = self.db.get_shared_key(event.id).await {
                let policy = Policy::decrypt_with_keys(&shared_key, &event.content)?;
                let mut nostr_pubkeys: Vec<XOnlyPublicKey> = Vec::new();
                for tag in event.tags.iter() {
                    if let Tag::PubKey(pubkey, ..) = tag {
                        nostr_pubkeys.push(*pubkey);
                    }
                }
                if nostr_pubkeys.is_empty() {
                    tracing::error!("Policy {} not contains any nostr pubkey", event.id);
                } else {
                    self.db
                        .save_policy(event.id, policy.clone(), nostr_pubkeys)
                        .await?;
                    self.manager.load_policy(event.id, policy).await?;
                    self.sync_channel
                        .send(Message::EventHandled(EventHandled::Policy(event.id)))?;
                }
            } else {
                self.db.save_pending_event(event.clone()).await?;
            }
        } else if event.kind == PROPOSAL_KIND
            && !self
                .db
                .exists(Type::Proposal {
                    proposal_id: event.id,
                })
                .await?
        {
            if let Some(policy_id) = util::extract_first_event_id(&event) {
                if let Ok(shared_key) = self.db.get_shared_key(policy_id).await {
                    let proposal = Proposal::decrypt_with_keys(&shared_key, &event.content)?;
                    self.db.save_proposal(event.id, policy_id, proposal).await?;
                    self.sync_channel
                        .send(Message::EventHandled(EventHandled::Proposal(event.id)))?;
                } else {
                    self.db.save_pending_event(event.clone()).await?;
                }
            } else {
                tracing::error!("Impossible to find policy id in proposal {}", event.id);
            }
        } else if event.kind == APPROVED_PROPOSAL_KIND
            && !self
                .db
                .exists(Type::ApprovedProposal {
                    approval_id: event.id,
                })
                .await?
        {
            if let Some(proposal_id) = util::extract_first_event_id(&event) {
                if let Some(Tag::Event(policy_id, ..)) =
                    util::extract_tags_by_kind(&event, TagKind::E).get(1)
                {
                    if let Ok(shared_key) = self.db.get_shared_key(*policy_id).await {
                        let approved_proposal =
                            ApprovedProposal::decrypt_with_keys(&shared_key, &event.content)?;
                        self.db
                            .save_approved_proposal(
                                proposal_id,
                                event.pubkey,
                                event.id,
                                approved_proposal,
                                event.created_at,
                            )
                            .await?;
                        self.sync_channel
                            .send(Message::EventHandled(EventHandled::Approval {
                                proposal_id,
                            }))?;
                    } else {
                        self.db.save_pending_event(event.clone()).await?;
                    }
                } else {
                    tracing::error!("Impossible to find policy id in proposal {}", event.id);
                }
            } else {
                tracing::error!(
                    "Impossible to find proposal id in approved proposal {}",
                    event.id
                );
            }
        } else if event.kind == COMPLETED_PROPOSAL_KIND
            && !self
                .db
                .exists(Type::CompletedProposal {
                    completed_proposal_id: event.id,
                })
                .await?
        {
            if let Some(proposal_id) = util::extract_first_event_id(&event) {
                self.db.delete_proposal(proposal_id).await?;
                if let Some(Tag::Event(policy_id, ..)) =
                    util::extract_tags_by_kind(&event, TagKind::E).get(1)
                {
                    if let Ok(shared_key) = self.db.get_shared_key(*policy_id).await {
                        let completed_proposal =
                            CompletedProposal::decrypt_with_keys(&shared_key, &event.content)?;

                        // Insert TX from completed proposal if the event was created in the last 60 secs
                        if event.created_at.add(Duration::from_secs(60)) >= Timestamp::now() {
                            if let CompletedProposal::Spending { tx, .. } = &completed_proposal {
                                match self
                                    .manager
                                    .insert_tx(
                                        *policy_id,
                                        tx.clone(),
                                        ConfirmationTime::Unconfirmed {
                                            last_seen: event.created_at.as_u64(),
                                        },
                                    )
                                    .await
                                {
                                    Ok(res) => {
                                        if res {
                                            tracing::info!(
                                                "Saved pending TX for finalized proposal {}",
                                                event.id
                                            );
                                        } else {
                                            tracing::warn!(
                                                "TX of finalized proposal {} already exists",
                                                event.id
                                            );
                                        }
                                    }
                                    Err(e) => tracing::error!(
                                        "Impossible to save TX from completed proposal {}: {e}",
                                        event.id
                                    ),
                                }
                            }
                        }

                        self.db
                            .save_completed_proposal(event.id, *policy_id, completed_proposal)
                            .await?;
                        self.sync_channel.send(Message::EventHandled(
                            EventHandled::CompletedProposal(event.id),
                        ))?;
                    } else {
                        self.db.save_pending_event(event.clone()).await?;
                    }
                } else {
                    tracing::error!(
                        "Impossible to find policy id in completed proposal {}",
                        event.id
                    );
                }
            }
        } else if event.kind == SIGNERS_KIND {
            let keys: Keys = self.keys().await;
            let signer = Signer::decrypt_with_keys(&keys, event.content)?;
            self.db.save_signer(event.id, signer).await?;
            self.sync_channel
                .send(Message::EventHandled(EventHandled::Signer(event.id)))?;
        } else if event.kind == SHARED_SIGNERS_KIND {
            let public_key =
                util::extract_first_public_key(&event).ok_or(Error::PublicKeyNotFound)?;
            let keys: Keys = self.keys().await;
            if event.pubkey == keys.public_key() {
                let signer_id =
                    util::extract_first_event_id(&event).ok_or(Error::SignerIdNotFound)?;
                self.db
                    .save_my_shared_signer(signer_id, event.id, public_key)
                    .await?;
                self.sync_channel
                    .send(Message::EventHandled(EventHandled::MySharedSigner(
                        event.id,
                    )))?;
            } else {
                let shared_signer =
                    nip04::decrypt(&keys.secret_key()?, &event.pubkey, event.content)?;
                let shared_signer = SharedSigner::from_json(shared_signer)?;
                self.db
                    .save_shared_signer(event.id, event.pubkey, shared_signer)
                    .await?;
                self.sync_channel
                    .send(Message::EventHandled(EventHandled::SharedSigner(event.id)))?;
            }
        } else if event.kind == LABELS_KIND {
            if let Some(policy_id) = util::extract_first_event_id(&event) {
                if let Some(identifier) = event.identifier() {
                    if let Ok(shared_key) = self.db.get_shared_key(policy_id).await {
                        let label = Label::decrypt_with_keys(&shared_key, &event.content)?;
                        self.db.save_label(identifier, policy_id, label).await?;
                        self.sync_channel
                            .send(Message::EventHandled(EventHandled::Label))?;
                    } else {
                        self.db.save_pending_event(event.clone()).await?;
                    }
                } else {
                    tracing::error!("Label identifier not found in event {}", event.id);
                }
            } else {
                tracing::error!("Impossible to find policy id in proposal {}", event.id);
            }
        } else if event.kind == Kind::EventDeletion {
            for tag in event.tags.iter() {
                if let Tag::Event(event_id, ..) = tag {
                    if let Ok(Event { pubkey, .. }) =
                        self.client.database().event_by_id(*event_id).await
                    {
                        if pubkey == event.pubkey {
                            self.db.delete_generic_event_id(*event_id).await?;
                        } else {
                            tracing::warn!(
                                "{pubkey} tried to delete an event not owned by him: {event_id}"
                            );
                        }
                    }
                }
            }
            self.sync_channel
                .send(Message::EventHandled(EventHandled::EventDeletion))?;
        } else if event.kind == Kind::ContactList {
            let pubkeys = event.public_keys().copied();
            let filter: Filter = Filter::new().authors(pubkeys).kind(Kind::Metadata);
            self.client
                .req_events_of(vec![filter], Some(Duration::from_secs(60)))
                .await;
            self.sync_channel
                .send(Message::EventHandled(EventHandled::Contacts))?;
        } else if event.kind == Kind::Metadata {
            self.sync_channel
                .send(Message::EventHandled(EventHandled::Metadata(event.pubkey)))?;
        } else if event.kind == Kind::RelayList {
            if event.pubkey == self.keys().await.public_key() {
                tracing::debug!("Received relay list: {:?}", event.tags);
                let current_relays: HashSet<Url> = self
                    .db
                    .get_relays(true)
                    .await?
                    .into_iter()
                    .map(|(url, ..)| url)
                    .collect();
                let list: HashSet<Url> = nip65::extract_relay_list(&event)
                    .into_iter()
                    .filter_map(|(url, ..)| Url::try_from(url).ok())
                    .collect();

                // Add relays
                for relay_url in list.difference(&current_relays) {
                    tracing::debug!("[relay list] Added {relay_url}");
                    self.add_relay_with_opts(relay_url.to_string(), None, false)
                        .await?;
                }

                // Remove relays
                for relay_url in current_relays.difference(&list) {
                    tracing::debug!("[relay list] Removed {relay_url}");
                    self.remove_relay_with_opts(relay_url.to_string(), false)
                        .await?;
                }

                self.sync_channel
                    .send(Message::EventHandled(EventHandled::RelayList))?;
            }
        } else if event.kind == KEY_AGENT_VERIFIED {
            let new_verified_agents: VerifiedKeyAgents = VerifiedKeyAgents::from_event(&event)?;
            let mut verified_key_agents = self.verified_key_agents.write().await;
            *verified_key_agents = new_verified_agents;

            // TODO: send notification
        } else if event.kind == Kind::NostrConnect
            && self.db.nostr_connect_session_exists(event.pubkey).await?
        {
            let keys: Keys = self.keys().await;
            let content = nip04::decrypt(&keys.secret_key()?, &event.pubkey, event.content)?;
            let msg = NIP46Message::from_json(content)?;
            if let Ok(request) = msg.to_request() {
                match request {
                    NIP46Request::Disconnect => {
                        self._disconnect_nostr_connect_session(event.pubkey, false)
                            .await?;
                    }
                    NIP46Request::GetPublicKey => {
                        let uri = self.db.get_nostr_connect_session(event.pubkey).await?;
                        let msg = msg
                            .generate_response(&keys)?
                            .ok_or(Error::CantGenerateNostrConnectResponse)?;
                        let nip46_event = EventBuilder::nostr_connect(&keys, uri.public_key, msg)?
                            .to_event(&keys)?;
                        // TODO: use send_event?
                        self.client
                            .pool()
                            .send_msg_to(uri.relay_url, ClientMessage::new_event(nip46_event), None)
                            .await?;
                    }
                    _ => {
                        if self
                            .db
                            .is_nostr_connect_session_pre_authorized(event.pubkey)
                            .await
                        {
                            let uri = self.db.get_nostr_connect_session(event.pubkey).await?;
                            let keys: Keys = self.keys().await;
                            let req_message = msg.clone();
                            let msg = msg
                                .generate_response(&keys)?
                                .ok_or(Error::CantGenerateNostrConnectResponse)?;
                            let nip46_event =
                                EventBuilder::nostr_connect(&keys, uri.public_key, msg)?
                                    .to_event(&keys)?;
                            self.client
                                .pool()
                                .send_msg_to(
                                    uri.relay_url,
                                    ClientMessage::new_event(nip46_event),
                                    None,
                                )
                                .await?;
                            self.db
                                .save_nostr_connect_request(
                                    event.id,
                                    event.pubkey,
                                    req_message,
                                    event.created_at,
                                    true,
                                )
                                .await?;
                            tracing::info!(
                                "Auto approved nostr connect request {} for app {}",
                                event.id,
                                event.pubkey
                            )
                        } else {
                            self.db
                                .save_nostr_connect_request(
                                    event.id,
                                    event.pubkey,
                                    msg,
                                    event.created_at,
                                    false,
                                )
                                .await?;
                            // TODO: save/send notification
                        }
                    }
                };
                self.sync_channel.send(Message::EventHandled(
                    EventHandled::NostrConnectRequest(event.id),
                ))?;
            }
        }

        Ok(())
    }
}
