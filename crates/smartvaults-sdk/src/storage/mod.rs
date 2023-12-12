// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::{HashMap, VecDeque};
use std::str::FromStr;
use std::sync::Arc;

use nostr_sdk::database::DynNostrDatabase;
use nostr_sdk::nips::nip04;
use nostr_sdk::{Client, Event, EventId, Filter, Keys, Tag, Timestamp};
use smartvaults_core::secp256k1::{SecretKey, XOnlyPublicKey};
use smartvaults_core::{ApprovedProposal, Policy, Proposal};
use smartvaults_protocol::v1::constants::{
    APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, LABELS_KIND, POLICY_KIND, PROPOSAL_KIND,
    SHARED_KEY_KIND, SIGNERS_KIND,
};
use smartvaults_protocol::v1::Encryption;
use tokio::sync::RwLock;

mod model;

pub(crate) use self::model::{InternalApproval, InternalPolicy, InternalProposal};
use crate::types::GetApprovedProposals;
use crate::{Error, EventHandled};

/// Smart Vaults In-Memory Storage
#[derive(Debug, Clone)]
pub(crate) struct SmartVaultsStorage {
    keys: Keys,
    shared_keys: Arc<RwLock<HashMap<EventId, Keys>>>,
    vaults: Arc<RwLock<HashMap<EventId, InternalPolicy>>>,
    proposals: Arc<RwLock<HashMap<EventId, InternalProposal>>>,
    approvals: Arc<RwLock<HashMap<EventId, InternalApproval>>>,
    pending: Arc<RwLock<VecDeque<Event>>>,
}

impl SmartVaultsStorage {
    /// Build storage from Nostr Database
    #[tracing::instrument(skip_all)]
    pub async fn build(client: &Client) -> Result<Self, Error> {
        let keys: Keys = client.keys().await;
        let database: Arc<DynNostrDatabase> = client.database();

        let this: Self = Self {
            keys,
            shared_keys: Arc::new(RwLock::new(HashMap::new())),
            vaults: Arc::new(RwLock::new(HashMap::new())),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            approvals: Arc::new(RwLock::new(HashMap::new())),
            pending: Arc::new(RwLock::new(VecDeque::new())),
        };

        let author_filter: Filter = Filter::new().author(this.keys.public_key()).kinds([
            SHARED_KEY_KIND,
            APPROVED_PROPOSAL_KIND,
            SIGNERS_KIND,
        ]);
        let pubkey_filter: Filter = Filter::new().pubkey(this.keys.public_key()).kinds([
            SHARED_KEY_KIND,
            POLICY_KIND,
            PROPOSAL_KIND,
            APPROVED_PROPOSAL_KIND,
            COMPLETED_PROPOSAL_KIND,
            LABELS_KIND,
        ]);
        for event in database
            .query(vec![author_filter, pubkey_filter])
            .await?
            .into_iter()
        {
            if let Err(e) = this.handle_event(&event).await {
                tracing::error!("Impossible to handle event: {e}");
            }
        }

        // Clone to avoid lock in handle event
        let pending = this.pending.read().await.clone();
        for event in pending.into_iter() {
            if let Err(e) = this.handle_event(&event).await {
                tracing::error!("Impossible to handle event: {e}");
            }
        }

        Ok(this)
    }

    async fn handle_event(&self, event: &Event) -> Result<Option<EventHandled>, Error> {
        if event.kind == SHARED_KEY_KIND {
            let policy_id = event
                .event_ids()
                .next()
                .copied()
                .ok_or(Error::PolicyNotFound)?;
            let mut shared_keys = self.shared_keys.write().await;
            if let HashMapEntry::Vacant(e) = shared_keys.entry(policy_id) {
                let content =
                    nip04::decrypt(&self.keys.secret_key()?, &event.pubkey, &event.content)?;
                let sk = SecretKey::from_str(&content)?;
                let shared_key = Keys::new(sk);
                e.insert(shared_key);
                return Ok(Some(EventHandled::SharedKey(event.id)));
            }
        } else if event.kind == POLICY_KIND {
            let shared_keys = self.shared_keys.read().await;
            let mut vaults = self.vaults.write().await;
            if let HashMapEntry::Vacant(e) = vaults.entry(event.id) {
                if let Some(shared_key) = shared_keys.get(&event.id) {
                    let policy = Policy::decrypt_with_keys(shared_key, &event.content)?;
                    let mut nostr_pubkeys: Vec<XOnlyPublicKey> = Vec::new();
                    for tag in event.tags.iter() {
                        if let Tag::PubKey(pubkey, ..) = tag {
                            nostr_pubkeys.push(*pubkey);
                        }
                    }
                    if nostr_pubkeys.is_empty() {
                        tracing::error!("Policy {} not contains any nostr pubkey", event.id);
                    } else {
                        e.insert(InternalPolicy {
                            policy,
                            public_keys: nostr_pubkeys,
                            last_sync: None,
                        });
                        return Ok(Some(EventHandled::Policy(event.id)));
                    }
                } else {
                    let mut pending = self.pending.write().await;
                    pending.push_back(event.clone());
                }
            }
        } else if event.kind == PROPOSAL_KIND {
            let shared_keys = self.shared_keys.read().await;
            let mut proposals = self.proposals.write().await;
            if let HashMapEntry::Vacant(e) = proposals.entry(event.id) {
                if let Some(policy_id) = event.event_ids().next() {
                    if let Some(shared_key) = shared_keys.get(policy_id) {
                        let proposal = Proposal::decrypt_with_keys(shared_key, &event.content)?;
                        e.insert(InternalProposal {
                            policy_id: *policy_id,
                            proposal,
                            timestamp: event.created_at,
                        });
                        return Ok(Some(EventHandled::Proposal(event.id)));
                    } else {
                        let mut pending = self.pending.write().await;
                        pending.push_back(event.clone());
                    }
                } else {
                    tracing::error!("Impossible to find policy id in proposal {}", event.id);
                }
            }
        } else if event.kind == APPROVED_PROPOSAL_KIND {
            let shared_keys = self.shared_keys.read().await;
            let mut approvals = self.approvals.write().await;
            if let HashMapEntry::Vacant(e) = approvals.entry(event.id) {
                let mut ids = event.event_ids();
                if let Some(proposal_id) = ids.next().copied() {
                    if let Some(policy_id) = ids.next() {
                        if let Some(shared_key) = shared_keys.get(policy_id) {
                            let approved_proposal =
                                ApprovedProposal::decrypt_with_keys(shared_key, &event.content)?;
                            e.insert(InternalApproval {
                                proposal_id,
                                policy_id: *policy_id,
                                public_key: event.pubkey,
                                approval: approved_proposal,
                                timestamp: event.created_at,
                            });
                            return Ok(Some(EventHandled::Approval { proposal_id }));
                        } else {
                            let mut pending = self.pending.write().await;
                            pending.push_back(event.clone());
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
            }
        } /* else if event.kind == COMPLETED_PROPOSAL_KIND
              && !self
                  .db
                  .exists(Type::CompletedProposal {
                      completed_proposal_id: event.id,
                  })
                  .await?
          {
              let mut ids = event.event_ids();
              if let Some(proposal_id) = ids.next().copied() {
                  self.db.delete_proposal(proposal_id).await?;
                  if let Some(policy_id) = ids.next() {
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
              let public_key = event.public_keys().next().ok_or(Error::PublicKeyNotFound)?;
              let keys: Keys = self.keys().await;
              if event.pubkey == keys.public_key() {
                  let signer_id = event.event_ids().next().ok_or(Error::SignerIdNotFound)?;
                  self.db
                      .save_my_shared_signer(*signer_id, event.id, *public_key)
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
              if let Some(policy_id) = event.event_ids().next().copied() {
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
          } */

        Ok(None)
    }

    pub async fn save_shared_key(&self, policy_id: EventId, shared_key: Keys) {
        let mut shared_keys = self.shared_keys.write().await;
        shared_keys.insert(policy_id, shared_key);
    }

    /// Get shared key
    pub async fn shared_key(&self, vault_id: &EventId) -> Result<Keys, Error> {
        let shared_keys = self.shared_keys.read().await;
        shared_keys.get(vault_id).cloned().ok_or(Error::NotFound)
    }

    pub async fn save_vault(&self, policy_id: EventId, internal: InternalPolicy) {
        let mut vaults = self.vaults.write().await;
        vaults.insert(policy_id, internal);
    }

    pub async fn delete_vault(&self, vault_id: &EventId) {
        let mut vaults = self.vaults.write().await;
        vaults.remove(vault_id);
    }

    /// Get vaults
    pub async fn vaults(&self) -> HashMap<EventId, InternalPolicy> {
        self.vaults
            .read()
            .await
            .iter()
            .map(|(id, internal)| (*id, internal.clone()))
            .collect()
    }

    /// Get [`Vault`]
    pub async fn vault(&self, vault_id: &EventId) -> Result<InternalPolicy, Error> {
        let vaults = self.vaults.read().await;
        vaults.get(vault_id).cloned().ok_or(Error::NotFound)
    }

    /// Updat last vault sync
    pub async fn update_last_sync(&self, vault_id: &EventId, last_sync: Option<Timestamp>) {
        let mut vaults = self.vaults.write().await;
        if let Some(internal) = vaults.get_mut(vault_id) {
            internal.last_sync = last_sync;
        }
    }

    pub async fn save_proposal(&self, proposal_id: EventId, internal: InternalProposal) {
        let mut proposals = self.proposals.write().await;
        proposals.insert(proposal_id, internal);
    }

    pub async fn delete_proposal(&self, proposal_id: &EventId) {
        let mut proposals = self.proposals.write().await;
        proposals.remove(proposal_id);
    }

    /// Get proposals
    pub async fn proposals(&self) -> HashMap<EventId, InternalProposal> {
        self.proposals
            .read()
            .await
            .iter()
            .map(|(id, internal)| (*id, internal.clone()))
            .collect()
    }

    pub async fn proposal(&self, proposal_id: &EventId) -> Result<InternalProposal, Error> {
        let proposals = self.proposals.read().await;
        proposals.get(proposal_id).cloned().ok_or(Error::NotFound)
    }

    pub async fn save_approval(&self, approval_id: EventId, internal: InternalApproval) {
        let mut approvals = self.approvals.write().await;
        approvals.insert(approval_id, internal);
    }

    pub async fn delete_approval(&self, approval_id: &EventId) {
        let mut approvals = self.approvals.write().await;
        approvals.remove(approval_id);
    }

    /// Get approvals
    pub async fn approvals(&self) -> HashMap<EventId, InternalApproval> {
        self.approvals
            .read()
            .await
            .iter()
            .map(|(id, internal)| (*id, internal.clone()))
            .collect()
    }

    pub async fn approval(&self, approval_id: &EventId) -> Result<InternalApproval, Error> {
        let approvals = self.approvals.read().await;
        approvals.get(approval_id).cloned().ok_or(Error::NotFound)
    }

    /// Approvals by proposal ID
    pub async fn approvals_by_proposal_id(
        &self,
        proposal_id: &EventId,
    ) -> Result<GetApprovedProposals, Error> {
        let InternalProposal {
            policy_id,
            proposal,
            ..
        } = self.proposal(proposal_id).await?;
        Ok(GetApprovedProposals {
            policy_id,
            proposal,
            approved_proposals: self
                .approvals
                .read()
                .await
                .values()
                .filter(|internal| internal.proposal_id == *proposal_id)
                .map(|internal| internal.approval.clone())
                .collect(),
        })
    }
}
