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
use nostr_sdk::prelude::*;
use smartvaults_core::bdk::chain::ConfirmationTime;
use smartvaults_core::bdk::FeeRate;
use smartvaults_core::bitcoin::Network;
use smartvaults_core::Priority;
use smartvaults_protocol::v1::constants::{
    KEY_AGENT_SIGNALING, KEY_AGENT_SIGNER_OFFERING_KIND, KEY_AGENT_VERIFIED,
    SMARTVAULTS_MAINNET_PUBLIC_KEY, SMARTVAULTS_TESTNET_PUBLIC_KEY,
};
use smartvaults_protocol::v2::{
    NostrPublicIdentifier, ProposalIdentifier, ProposalType, VaultIdentifier,
};
use tokio::sync::broadcast::Receiver;

use super::{Error, SmartVaults};
use crate::constants::WALLET_SYNC_INTERVAL;
use crate::storage::InternalVault;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EventHandled {
    SharedKey(EventId),
    Vault(VaultIdentifier),
    VaultMetadata(VaultIdentifier),
    VaultInvite(VaultIdentifier),
    Proposal(ProposalIdentifier),
    Approval {
        vault_id: VaultIdentifier,
        proposal_id: ProposalIdentifier,
    },
    CompletedProposal(EventId),
    Signer(EventId),
    SharedSigner(EventId),
    SharedSignerInvite(NostrPublicIdentifier),
    Contacts,
    Metadata(PublicKey),
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
    WalletSyncCompleted(VaultIdentifier),
    BlockHeightUpdated,
    MempoolFeesUpdated(BTreeMap<Priority, FeeRate>),
}

impl SmartVaults {
    fn block_height_syncer(&self) -> Result<AbortHandle, Error> {
        let this = self.clone();
        Ok(thread::abortable(async move {
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
        })?)
    }

    fn mempool_fees_syncer(&self) -> Result<AbortHandle, Error> {
        let this = self.clone();
        Ok(thread::abortable(async move {
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
        })?)
    }

    fn policies_syncer(&self) -> Result<AbortHandle, Error> {
        let this = self.clone();
        Ok(thread::abortable(async move {
            loop {
                match this.config.electrum_endpoint().await {
                    Ok(endpoint) => {
                        let proxy = this.config.proxy().await.ok();
                        if let Err(e) = this
                            .manager
                            .sync_all(endpoint, proxy, Some(this.sync_channel.clone()))
                            .await
                        {
                            tracing::error!("Impossible to sync all wallets: {e}");
                        }
                    }
                    Err(e) => tracing::error!("Impossible to sync wallets: {e}"),
                }

                thread::sleep(WALLET_SYNC_INTERVAL).await;
            }
        })?)
    }

    fn vaults_authored_filter_resubscribe(&self) -> Result<AbortHandle, Error> {
        let this = self.clone();
        Ok(thread::abortable(async move {
            loop {
                if this.resubscribe_vaults.load(Ordering::SeqCst) {
                    // Resubscribe to vaults authored filter
                    for (relay_url, relay) in this.client.relays().await {
                        let last_sync: Timestamp =
                            match this.db.get_last_relay_sync(relay_url.clone()).await {
                                Ok(ts) => ts,
                                Err(e) => {
                                    tracing::error!("Impossible to get last relay sync: {e}");
                                    Timestamp::from(0)
                                }
                            };

                        let filters: Vec<Filter> = this.sync_vaults_filter(last_sync).await;
                        if let Err(e) = relay
                            .subscribe_with_internal_id(
                                InternalSubscriptionId::from("smartvaults-vaults-authored"),
                                filters,
                                RelaySendOptions::new(),
                            )
                            .await
                        {
                            tracing::error!(
                                "Impossible to subscribe to {relay_url} [vaults-authored]: {e}"
                            );
                        }
                    }

                    this.set_resubscribe_vaults(false);
                }

                thread::sleep(Duration::from_secs(10)).await;
            }
        })?)
    }

    /// Update vault-authored subscription filters
    pub(crate) fn set_resubscribe_vaults(&self, value: bool) {
        let _ = self
            .resubscribe_vaults
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(value));
    }

    pub fn sync_notifications(&self) -> Receiver<Message> {
        self.sync_channel.subscribe()
    }

    /// Get [Filter] for everything authored by vaults shared key
    pub(crate) async fn sync_vaults_filter(&self, since: Timestamp) -> Vec<Filter> {
        let vaults = self.storage.vaults().await;
        let public_keys = vaults.into_values().map(|i| {
            let secret_key = i.shared_key();
            let keys = Keys::new(secret_key.clone());
            keys.public_key()
        });
        vec![Filter::new().authors(public_keys).since(since)]
    }

    pub(crate) async fn sync_filters(&self, since: Timestamp) -> Vec<Filter> {
        let public_key: PublicKey = self.keys.public_key();

        // Author filter include vaults, metadata, contacts, relay list, ...
        let author_filter: Filter = Filter::new().author(public_key).since(since);

        // Pubkey filter include invites, nostr connect, ...
        let pubkey_filter: Filter = Filter::new().pubkey(public_key).since(since);

        let key_agents: Filter = Filter::new()
            .kinds([KEY_AGENT_SIGNALING, KEY_AGENT_SIGNER_OFFERING_KIND])
            .since(since);
        let smartvaults: Filter = Filter::new()
            .author(match self.network {
                Network::Bitcoin => *SMARTVAULTS_MAINNET_PUBLIC_KEY,
                _ => *SMARTVAULTS_TESTNET_PUBLIC_KEY,
            })
            .kind(KEY_AGENT_VERIFIED);

        let mut filters: Vec<Filter> = vec![author_filter, pubkey_filter, key_agents, smartvaults];

        let contacts: Vec<PublicKey> = self
            .client
            .database()
            .contacts_public_keys(public_key)
            .await
            .unwrap_or_default();
        if !contacts.is_empty() {
            filters.push(Filter::new().authors(contacts).since(since));
        }

        filters
    }

    pub(crate) fn sync(&self) -> Result<(), Error> {
        if self.syncing.load(Ordering::SeqCst) {
            tracing::warn!("Syncing threads are already running");
        } else {
            let _ = self
                .syncing
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(true));
            let this = self.clone();
            thread::spawn(async move {
                // Sync timechain
                let block_height_syncer: AbortHandle = this.block_height_syncer()?;
                let mempool_fees_syncer: AbortHandle = this.mempool_fees_syncer()?;
                let policies_syncer: AbortHandle = this.policies_syncer()?;
                let vaults_authored_filter: AbortHandle =
                    this.vaults_authored_filter_resubscribe()?;

                for (relay_url, relay) in this.client.relays().await {
                    let last_sync: Timestamp =
                        match this.db.get_last_relay_sync(relay_url.clone()).await {
                            Ok(ts) => ts,
                            Err(e) => {
                                tracing::error!("Impossible to get last relay sync: {e}");
                                Timestamp::from(0)
                            }
                        };

                    // Subscribe to default filters
                    let filters: Vec<Filter> = this.sync_filters(last_sync).await;
                    if let Err(e) = relay
                        .subscribe_with_internal_id(
                            InternalSubscriptionId::from("smartvaults-default"),
                            filters,
                            RelaySendOptions::new(),
                        )
                        .await
                    {
                        tracing::error!("Impossible to subscribe to {relay_url} [default]: {e}");
                    }

                    // Subscribe to vaults authored filters
                    let filters: Vec<Filter> = this.sync_vaults_filter(last_sync).await;
                    if let Err(e) = relay
                        .subscribe_with_internal_id(
                            InternalSubscriptionId::from("smartvaults-vaults-authored"),
                            filters,
                            RelaySendOptions::new(),
                        )
                        .await
                    {
                        tracing::error!(
                            "Impossible to subscribe to {relay_url} [vaults-authored]: {e}"
                        );
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
                                vaults_authored_filter.abort();
                                let _ = this.syncing.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(false));
                            }
                        }

                        Ok(false)
                    })
                    .await;
                tracing::debug!("Exited from nostr sync thread");

                Ok::<(), Error>(())
            })?;

            // Negentropy reconciliation
            let this = self.clone();
            thread::spawn(async move {
                let opts = NegentropyOptions::new().direction(NegentropyDirection::Both);
                for filter in this.sync_filters(Timestamp::from(0)).await.into_iter() {
                    this.client.reconcile(filter, opts).await.unwrap();
                }
            })?;
        }

        Ok(())
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
            let content = nip04::decrypt(keys.secret_key()?, event.author_ref(), event.content())?;
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
                EventHandled::Vault(vault_id) => {
                    let InternalVault { vault, .. } = self.storage.vault(&vault_id).await?;
                    self.manager.load_policy(vault_id, vault.policy()).await?;

                    // Resubscribe to vaults authored filter
                    self.set_resubscribe_vaults(true);
                }
                EventHandled::Proposal(proposal_id) => {
                    let proposal = self.storage.proposal(&proposal_id).await?;
                    // Insert TX from completed proposal if the event was created in the last 60 secs
                    if proposal.is_finalized()
                        && event.created_at.add(Duration::from_secs(60)) >= Timestamp::now()
                    {
                        if let ProposalType::Spending | ProposalType::KeyAgentPayment =
                            proposal.r#type()
                        {
                            match self
                                .manager
                                .insert_tx(
                                    &proposal.vault_id(),
                                    proposal.tx().clone(),
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
            let _ = self.sync_channel.send(Message::EventHandled(h));
        }

        Ok(())
    }
}
