// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};
use std::ops::Add;
use std::sync::atomic::Ordering;
use std::time::Duration;

use async_utility::thread;
use futures_util::stream::AbortHandle;
use nostr_sdk::database::NostrDatabaseExt;
use nostr_sdk::nips::nip46::{Message as NIP46Message, Request as NIP46Request};
use nostr_sdk::nips::{nip04, nip65};
use nostr_sdk::{
    ClientMessage, Event, EventBuilder, EventId, Filter, JsonUtil, Keys, Kind, NegentropyDirection,
    NegentropyOptions, RelayMessage, RelayPoolNotification, RelaySendOptions, Result, Timestamp,
    Url,
};
use smartvaults_core::bdk::chain::ConfirmationTime;
use smartvaults_core::bdk::FeeRate;
use smartvaults_core::bitcoin::secp256k1::XOnlyPublicKey;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::{CompletedProposal, Priority};
use smartvaults_protocol::v1::constants::{
    APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, KEY_AGENT_SIGNALING,
    KEY_AGENT_SIGNER_OFFERING_KIND, KEY_AGENT_VERIFIED, LABELS_KIND, POLICY_KIND, PROPOSAL_KIND,
    SHARED_KEY_KIND, SHARED_SIGNERS_KIND, SIGNERS_KIND, SMARTVAULTS_MAINNET_PUBLIC_KEY,
    SMARTVAULTS_TESTNET_PUBLIC_KEY,
};
use tokio::sync::broadcast::Receiver;

use super::{Error, SmartVaults};
use crate::storage::{InternalCompletedProposal, InternalPolicy};

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
    VerifiedKeyAgents,
}

#[derive(Debug, Clone)]
pub enum Message {
    EventHandled(EventHandled),
    WalletSyncCompleted(EventId),
    BlockHeightUpdated,
    MempoolFeesUpdated(BTreeMap<Priority, FeeRate>),
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

    fn mempool_fees_syncer(&self) -> AbortHandle {
        let this = self.clone();
        thread::abortable(async move {
            loop {
                match this.config.electrum_endpoint().await {
                    Ok(endpoint) => {
                        let proxy = this.config.proxy().await.ok();
                        match this.manager.sync_mempool_fees(endpoint, proxy).await {
                            Ok(Some(fees)) => {
                                let _ = this.sync_channel.send(Message::MempoolFeesUpdated(fees));
                            }
                            Ok(None) => (),
                            Err(e) => tracing::error!("Impossible to get mempool fees: {e}"),
                        }
                    }
                    Err(e) => tracing::error!("Impossible to get mempool fees: {e}"),
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
                    Ok(endpoint) => {
                        let proxy = this.config.proxy().await.ok();
                        this.manager
                            .sync_all(endpoint, proxy, Some(this.sync_channel.clone()))
                            .await;
                    }
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
                for event in this.storage.pending_events().await.into_iter() {
                    let event_id = event.id;
                    if let Err(e) = this.handle_event(event).await {
                        tracing::error!("Impossible to handle pending event {event_id}: {e}");
                    }
                }
                thread::sleep(Duration::from_secs(30)).await;
            }
        })
    }

    pub fn sync_notifications(&self) -> Receiver<Message> {
        self.sync_channel.subscribe()
    }

    pub(crate) async fn sync_filters(&self, since: Timestamp) -> Vec<Filter> {
        let base_filter = Filter::new().kinds([
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

        let keys: &Keys = self.keys();
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
            .kinds([Kind::Metadata, Kind::ContactList, Kind::RelayList])
            .since(since);
        let key_agents: Filter = Filter::new()
            .kinds([KEY_AGENT_SIGNALING, KEY_AGENT_SIGNER_OFFERING_KIND])
            .since(since);
        let smartvaults: Filter = Filter::new()
            .author(match self.network {
                Network::Bitcoin => *SMARTVAULTS_MAINNET_PUBLIC_KEY,
                _ => *SMARTVAULTS_TESTNET_PUBLIC_KEY,
            })
            .kind(KEY_AGENT_VERIFIED);

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
                let mempool_fees_syncer: AbortHandle = this.mempool_fees_syncer();
                let policies_syncer: AbortHandle = this.policies_syncer();

                // Pending events handler
                let pending_event_handler = this.handle_pending_events();

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
                    if let Err(e) = relay.subscribe(filters, RelaySendOptions::new()).await {
                        tracing::error!("Impossible to subscribe to {relay_url}: {e}");
                    }
                }

                let _ = this
                    .client
                    .handle_notifications(|notification| async {
                        match notification {
                            RelayPoolNotification::Event { event, ..} => {
                                let event_id = event.id;
                                if event.is_expired() {
                                    tracing::warn!("Event {event_id} expired");
                                } else if let Err(e) = this.handle_event(event).await {
                                    tracing::error!("Impossible to handle event {event_id}: {e}");
                                }
                            }
                            RelayPoolNotification::Message { relay_url, message } => {
                                if let RelayMessage::EndOfStoredEvents(subscription_id) = message {
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
                                mempool_fees_syncer.abort();
                                policies_syncer.abort();
                                pending_event_handler.abort();
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
                let opts = NegentropyOptions::new().direction(NegentropyDirection::Both);
                for filter in this.sync_filters(Timestamp::from(0)).await.into_iter() {
                    this.client.reconcile(filter, opts).await.unwrap();
                }
            });
        }
    }

    async fn handle_event(&self, event: Event) -> Result<()> {
        if event.kind == Kind::ContactList {
            let pubkeys = event.public_keys().copied();
            let filter: Filter = Filter::new().authors(pubkeys).kind(Kind::Metadata);
            self.client
                .req_events_of(vec![filter], Some(Duration::from_secs(60)))
                .await;
            self.sync_channel
                .send(Message::EventHandled(EventHandled::Contacts))?;
        } else if event.kind == Kind::Metadata {
            self.sync_channel
                .send(Message::EventHandled(EventHandled::Metadata(
                    event.author(),
                )))?;
        } else if event.kind == Kind::RelayList {
            if event.author() == self.keys().public_key() {
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
        } else if event.kind == Kind::NostrConnect
            && self.db.nostr_connect_session_exists(event.author()).await?
        {
            let keys: &Keys = self.keys();
            let content = nip04::decrypt(&keys.secret_key()?, event.author_ref(), event.content())?;
            let msg = NIP46Message::from_json(content)?;
            if let Ok(request) = msg.to_request() {
                match request {
                    NIP46Request::Disconnect => {
                        self._disconnect_nostr_connect_session(event.author(), false)
                            .await?;
                    }
                    NIP46Request::GetPublicKey => {
                        let uri = self.db.get_nostr_connect_session(event.author()).await?;
                        let msg = msg
                            .generate_response(keys)?
                            .ok_or(Error::CantGenerateNostrConnectResponse)?;
                        let nip46_event = EventBuilder::nostr_connect(keys, uri.public_key, msg)?
                            .to_event(keys)?;
                        // TODO: use send_event?
                        self.client
                            .pool()
                            .send_msg_to(
                                [uri.relay_url],
                                ClientMessage::event(nip46_event),
                                RelaySendOptions::new().skip_send_confirmation(true),
                            )
                            .await?;
                    }
                    _ => {
                        if self
                            .db
                            .is_nostr_connect_session_pre_authorized(event.author())
                            .await
                        {
                            let uri = self.db.get_nostr_connect_session(event.author()).await?;
                            let keys: &Keys = self.keys();
                            let req_message = msg.clone();
                            let msg = msg
                                .generate_response(keys)?
                                .ok_or(Error::CantGenerateNostrConnectResponse)?;
                            let nip46_event =
                                EventBuilder::nostr_connect(keys, uri.public_key, msg)?
                                    .to_event(keys)?;
                            self.client
                                .pool()
                                .send_msg_to(
                                    [uri.relay_url],
                                    ClientMessage::event(nip46_event),
                                    RelaySendOptions::new().skip_send_confirmation(true),
                                )
                                .await?;
                            self.db
                                .save_nostr_connect_request(
                                    event.id,
                                    event.author(),
                                    req_message,
                                    event.created_at,
                                    true,
                                )
                                .await?;
                            tracing::info!(
                                "Auto approved nostr connect request {} for app {}",
                                event.id,
                                event.author()
                            )
                        } else {
                            self.db
                                .save_nostr_connect_request(
                                    event.id,
                                    event.author(),
                                    msg,
                                    event.created_at,
                                    false,
                                )
                                .await?;
                        }
                    }
                };
                self.sync_channel.send(Message::EventHandled(
                    EventHandled::NostrConnectRequest(event.id),
                ))?;
            }
        } else if let Some(h) = self.storage.handle_event(&event).await? {
            match h {
                EventHandled::Policy(vault_id) => {
                    let InternalPolicy { policy, .. } = self.storage.vault(&vault_id).await?;
                    self.manager.load_policy(event.id, policy).await?;
                }
                EventHandled::CompletedProposal(completed_proposal_id) => {
                    let InternalCompletedProposal {
                        policy_id,
                        proposal,
                        ..
                    } = self
                        .storage
                        .completed_proposal(&completed_proposal_id)
                        .await?;
                    // Insert TX from completed proposal if the event was created in the last 60 secs
                    if event.created_at.add(Duration::from_secs(60)) >= Timestamp::now() {
                        if let CompletedProposal::Spending { tx, .. } = proposal {
                            match self
                                .manager
                                .insert_tx(
                                    policy_id,
                                    tx,
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
                }
                _ => (),
            };
            self.sync_channel.send(Message::EventHandled(h))?;
        }

        Ok(())
    }
}
