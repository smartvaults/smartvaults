// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;
use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::sync::Arc;

use nostr_sdk::prelude::*;
use smartvaults_core::bitcoin::{Network, OutPoint, ScriptBuf, Txid};
use smartvaults_protocol::v1::constants::{
    APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, KEY_AGENT_VERIFIED, LABELS_KIND, POLICY_KIND,
    PROPOSAL_KIND, SHARED_KEY_KIND, SHARED_SIGNERS_KIND, SIGNERS_KIND,
    SMARTVAULTS_MAINNET_PUBLIC_KEY, SMARTVAULTS_TESTNET_PUBLIC_KEY,
};
use smartvaults_protocol::v1::{Encryption, Label, LabelData, LabelKind, VerifiedKeyAgents};
use smartvaults_protocol::v2::constants::{
    APPROVAL_KIND_V2, PROPOSAL_KIND_V2, SIGNER_KIND_V2, VAULT_KIND_V2,
};
use smartvaults_protocol::v2::{
    Approval, Proposal, ProposalIdentifier, ProtocolEncryption, SharedSigner, Signer,
    SignerIdentifier, Vault, VaultIdentifier,
};
use tokio::sync::RwLock;

mod model;

pub(crate) use self::model::{InternalApproval, InternalLabel, InternalVault};
use crate::types::GetApprovedProposals;
use crate::{Error, EventHandled};

#[derive(Debug, Clone, PartialEq, Eq)]
struct WrappedEvent {
    inner: Event,
}

impl PartialOrd for WrappedEvent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WrappedEvent {
    fn cmp(&self, other: &Self) -> Ordering {
        if self.inner.created_at != other.inner.created_at {
            self.inner.created_at.cmp(&other.inner.created_at)
        } else {
            self.inner.id.cmp(&other.inner.id)
        }
    }
}

/// Smart Vaults In-Memory Storage
#[derive(Debug, Clone)]
pub(crate) struct SmartVaultsStorage {
    keys: Keys,
    database: Arc<DynNostrDatabase>,
    vaults_ids: Arc<RwLock<HashMap<EventId, VaultIdentifier>>>,
    vaults_keys: Arc<RwLock<HashMap<PublicKey, Keys>>>,
    vaults: Arc<RwLock<HashMap<VaultIdentifier, InternalVault>>>,
    proposals_ids: Arc<RwLock<HashMap<EventId, ProposalIdentifier>>>,
    proposals: Arc<RwLock<HashMap<ProposalIdentifier, Proposal>>>,
    approvals: Arc<RwLock<HashMap<EventId, InternalApproval>>>,
    signers: Arc<RwLock<HashMap<SignerIdentifier, Signer>>>,
    my_shared_signers: Arc<RwLock<HashMap<SignerIdentifier, (EventId, PublicKey)>>>, /* Signer ID, Shared Signer ID, pubkey */
    shared_signers: Arc<RwLock<HashMap<EventId, SharedSigner>>>,
    labels: Arc<RwLock<HashMap<String, InternalLabel>>>,
    frozed_utxos: Arc<RwLock<HashMap<VaultIdentifier, HashSet<OutPoint>>>>,
    verified_key_agents: Arc<RwLock<VerifiedKeyAgents>>,
    pending: Arc<RwLock<BTreeSet<Event>>>,
}

impl SmartVaultsStorage {
    /// Build storage from Nostr Database
    #[tracing::instrument(skip_all)]
    pub async fn build(
        keys: Keys,
        database: Arc<DynNostrDatabase>,
        network: Network,
    ) -> Result<Self, Error> {
        let this: Self = Self {
            keys,
            database,
            vaults_ids: Arc::new(RwLock::new(HashMap::new())),
            vaults_keys: Arc::new(RwLock::new(HashMap::new())),
            vaults: Arc::new(RwLock::new(HashMap::new())),
            proposals_ids: Arc::new(RwLock::new(HashMap::new())),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            approvals: Arc::new(RwLock::new(HashMap::new())),
            signers: Arc::new(RwLock::new(HashMap::new())),
            my_shared_signers: Arc::new(RwLock::new(HashMap::new())),
            shared_signers: Arc::new(RwLock::new(HashMap::new())),
            labels: Arc::new(RwLock::new(HashMap::new())),
            frozed_utxos: Arc::new(RwLock::new(HashMap::new())),
            verified_key_agents: Arc::new(RwLock::new(VerifiedKeyAgents::empty(network))),
            pending: Arc::new(RwLock::new(BTreeSet::new())),
        };

        let author_filter: Filter = Filter::new().author(this.keys.public_key()).kinds([
            SHARED_KEY_KIND,
            POLICY_KIND,
            PROPOSAL_KIND,
            APPROVED_PROPOSAL_KIND,
            COMPLETED_PROPOSAL_KIND,
            SIGNERS_KIND,
            SHARED_SIGNERS_KIND,
            LABELS_KIND,
        ]);
        let pubkey_filter: Filter = Filter::new().pubkey(this.keys.public_key()).kinds([
            SHARED_KEY_KIND,
            POLICY_KIND,
            PROPOSAL_KIND,
            APPROVED_PROPOSAL_KIND,
            COMPLETED_PROPOSAL_KIND,
            SIGNERS_KIND,
            SHARED_SIGNERS_KIND,
            LABELS_KIND,
        ]);
        let smartvaults: Filter = Filter::new()
            .author(match network {
                Network::Bitcoin => *SMARTVAULTS_MAINNET_PUBLIC_KEY,
                _ => *SMARTVAULTS_TESTNET_PUBLIC_KEY,
            })
            .kind(KEY_AGENT_VERIFIED);

        let mut pending = this.pending.write().await;
        for event in this
            .database
            .query(vec![author_filter, pubkey_filter, smartvaults], Order::Asc)
            .await?
            .into_iter()
        {
            if let Err(e) = this.internal_handle_event(&mut pending, &event).await {
                tracing::error!("Impossible to handle event: {e}");
            }
        }

        // Clone to avoid lock in handle event
        for event in pending.clone().into_iter() {
            if let Err(e) = this.internal_handle_event(&mut pending, &event).await {
                tracing::error!("Impossible to handle event: {e}");
            }
        }

        drop(pending);

        Ok(this)
    }

    pub(crate) async fn handle_event(&self, event: &Event) -> Result<Option<EventHandled>, Error> {
        let mut pending = self.pending.write().await;
        self.internal_handle_event(&mut pending, event).await
    }

    async fn internal_handle_event(
        &self,
        pending: &mut BTreeSet<Event>,
        event: &Event,
    ) -> Result<Option<EventHandled>, Error> {
        if pending.contains(event) {
            pending.remove(event);
        }

        if event.kind == VAULT_KIND_V2 {
            let mut vaults_ids = self.vaults_ids.write().await;
            let mut vaults_keys = self.vaults_keys.write().await;
            let mut vaults = self.vaults.write().await;
            if let HashMapEntry::Vacant(e) = vaults_ids.entry(event.id) {
                let vault: Vault = Vault::decrypt_with_keys(&self.keys, &event.content)?;
                let vault_id = vault.id();
                let keys = Keys::new(vault.shared_key());
                e.insert(vault_id);
                vaults_keys.insert(keys.public_key(), keys);
                vaults.insert(vault_id, InternalVault { vault });
                return Ok(Some(EventHandled::Vault(vault_id)));
            }
        } else if event.kind == PROPOSAL_KIND_V2 {
            let mut vaults_keys = self.vaults_keys.write().await;
            let mut proposals_ids = self.proposals_ids.write().await;
            let mut proposals = self.proposals.write().await;
            if let HashMapEntry::Vacant(e) = proposals_ids.entry(event.id) {
                if let Some(shared_key) = vaults_keys.get(&event.pubkey) {
                    // Decrypt proposal
                    let proposal: Proposal =
                        Proposal::decrypt_with_keys(shared_key, &event.content)?;
                    let proposal_id = proposal.id();

                    // Froze UTXOs
                    if let Some(psbt) = proposal.psbt() {
                        self.freeze_utxos(
                            proposal.vault_id(),
                            psbt.unsigned_tx
                                .input
                                .iter()
                                .map(|txin| txin.previous_output),
                        )
                        .await;
                    }

                    // Insert proposal
                    e.insert(proposal_id);
                    proposals.insert(proposal_id, proposal);

                    return Ok(Some(EventHandled::Proposal(proposal_id)));
                } else {
                    pending.insert(event.clone());
                }
            }
        } else if event.kind == APPROVAL_KIND_V2 {
            let mut vaults_keys = self.vaults_keys.write().await;
            let mut approvals = self.approvals.write().await;
            if let HashMapEntry::Vacant(e) = approvals.entry(event.id) {
                if let Some(shared_public_key) = event.public_keys().next() {
                    if let Some(shared_key) = vaults_keys.get(shared_public_key) {
                        let approval = Approval::decrypt_with_keys(shared_key, &event.content)?;
                        let vault_id = approval.vault_id();
                        let proposal_id = approval.proposal_id();
                        e.insert(InternalApproval {
                            public_key: event.author(),
                            approval,
                            timestamp: event.created_at,
                        });
                        return Ok(Some(EventHandled::Approval {
                            vault_id,
                            proposal_id,
                        }));
                    } else {
                        pending.insert(event.clone());
                    }
                } else {
                    tracing::error!(
                        "Impossible to find shared_key public key in approval tags {}",
                        event.id
                    );
                }
            }
        } else if event.kind == SIGNER_KIND_V2 {
            let mut signers = self.signers.write().await;
            let signer = Signer::decrypt_with_keys(&self.keys, &event.content)?;
            if let HashMapEntry::Vacant(e) = signers.entry(signer.id()) {
                e.insert(signer);
            }
            return Ok(Some(EventHandled::Signer(event.id)));
        } else if event.kind == SHARED_SIGNERS_KIND {
            if event.author() == self.keys.public_key() {
                let signer_id = event.identifier().ok_or(Error::SignerIdNotFound)?;
                let public_key = event.public_keys().next().ok_or(Error::PublicKeyNotFound)?;

                let mut my_shared_signers = self.my_shared_signers.write().await;
                if let HashMapEntry::Vacant(e) = my_shared_signers.entry(signer_id) {
                    e.insert((event.id, *public_key));
                    return Ok(Some(EventHandled::MySharedSigner(event.id)));
                }
            } else {
                let mut shared_signers = self.shared_signers.write().await;
                if let HashMapEntry::Vacant(e) = shared_signers.entry(event.id) {
                    let shared_signer: String = nip04::decrypt(
                        self.keys.secret_key()?,
                        event.author_ref(),
                        &event.content,
                    )?;
                    let shared_signer: SharedSigner = SharedSigner::from_json(shared_signer)?;
                    e.insert(shared_signer);
                    return Ok(Some(EventHandled::SharedSigner(event.id)));
                }
            }
        } else if event.kind == LABELS_KIND {
            let mut labels = self.labels.write().await;
            let shared_keys = self.shared_keys.read().await;
            if let Some(policy_id) = event.event_ids().next() {
                if let Some(identifier) = event.identifier() {
                    if let Some(shared_key) = shared_keys.get(policy_id) {
                        let label = Label::decrypt_with_keys(shared_key, &event.content)?;
                        labels.insert(
                            identifier.to_string(),
                            InternalLabel {
                                vault_id: *vault_id,
                                label,
                            },
                        );
                        return Ok(Some(EventHandled::Label));
                    } else {
                        pending.insert(event.clone());
                    }
                } else {
                    tracing::error!("Label identifier not found in event {}", event.id);
                }
            } else {
                tracing::error!("Impossible to find policy id in proposal {}", event.id);
            }
        } else if event.kind == Kind::EventDeletion {
            for event_id in event.event_ids() {
                if let Ok(true) = self.database.has_event_id_been_deleted(event_id).await {
                    self.delete_event(event_id).await;
                    return Ok(Some(EventHandled::EventDeletion));
                } else {
                    tracing::error!("Event {event_id} not deleted");
                }
            }

            for coordinate in event.coordinates() {
                if let Ok(true) = self
                    .database
                    .has_coordinate_been_deleted(&coordinate, event.created_at)
                    .await
                {
                    let filter: Filter = coordinate.into();
                    let filter: Filter = filter.until(event.created_at);
                    let event_ids = self
                        .database
                        .event_ids_by_filters(vec![filter], Order::Desc)
                        .await?;
                    for event_id in event_ids.into_iter() {
                        self.delete_event(&event_id).await;
                    }
                    return Ok(Some(EventHandled::EventDeletion));
                }
            }
        } else if event.kind == KEY_AGENT_VERIFIED {
            let new_verified_agents: VerifiedKeyAgents = VerifiedKeyAgents::from_event(event)?;
            let mut verified_key_agents = self.verified_key_agents.write().await;
            *verified_key_agents = new_verified_agents;
            return Ok(Some(EventHandled::VerifiedKeyAgents));
        }

        Ok(None)
    }

    pub async fn pending_events(&self) -> BTreeSet<Event> {
        self.pending.read().await.clone()
    }

    /// Delete event without know the kind
    pub async fn delete_event(&self, event_id: &EventId) {
        if self.delete_approval(event_id).await {
            return;
        }

        // TODO: delete proposal, signer, ...

        self.delete_shared_signer(event_id).await;
    }

    pub async fn save_vault(&self, vault_id: VaultIdentifier, internal: InternalVault) {
        let mut vaults = self.vaults.write().await;
        vaults.insert(vault_id, internal);
    }

    pub async fn delete_vault(&self, vault_id: &VaultIdentifier) -> bool {
        let mut vaults = self.vaults.write().await;
        vaults.remove(vault_id).is_some()
    }

    /// Get vaults
    pub async fn vaults(&self) -> HashMap<VaultIdentifier, InternalVault> {
        self.vaults
            .read()
            .await
            .iter()
            .map(|(id, internal)| (*id, internal.clone()))
            .collect()
    }

    /// Get [`Vault`]
    pub async fn vault(&self, vault_id: &VaultIdentifier) -> Result<InternalVault, Error> {
        let vaults = self.vaults.read().await;
        vaults.get(vault_id).cloned().ok_or(Error::NotFound)
    }

    pub async fn save_proposal(&self, proposal_id: ProposalIdentifier, proposal: Proposal) {
        let mut proposals = self.proposals.write().await;
        proposals.insert(proposal_id, proposal);
    }

    pub async fn delete_proposal(&self, proposal_id: &ProposalIdentifier) -> bool {
        let mut proposals = self.proposals.write().await;
        proposals.remove(proposal_id).is_some()
    }

    /// Get proposals
    pub async fn proposals(&self) -> HashMap<ProposalIdentifier, Proposal> {
        self.proposals
            .read()
            .await
            .iter()
            .map(|(id, proposal)| (*id, proposal.clone()))
            .collect()
    }

    pub async fn proposal(&self, proposal_id: &ProposalIdentifier) -> Result<Proposal, Error> {
        let proposals = self.proposals.read().await;
        proposals.get(proposal_id).cloned().ok_or(Error::NotFound)
    }

    pub async fn save_approval(&self, approval_id: EventId, internal: InternalApproval) {
        let mut approvals = self.approvals.write().await;
        approvals.insert(approval_id, internal);
    }

    pub async fn delete_approval(&self, approval_id: &EventId) -> bool {
        let mut approvals = self.approvals.write().await;
        approvals.remove(approval_id).is_some()
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
        proposal_id: &ProposalIdentifier,
    ) -> Result<GetApprovedProposals, Error> {
        let proposal = self.proposal(proposal_id).await?;
        Ok(GetApprovedProposals {
            proposal,
            approvals: self
                .approvals
                .read()
                .await
                .values()
                .filter(|internal| internal.approval.proposal_id() == *proposal_id)
                .map(|internal| internal.approval.clone())
                .collect(),
        })
    }

    pub async fn description_by_txid(
        &self,
        vault_id: VaultIdentifier,
        txid: Txid,
    ) -> Option<String> {
        let proposals = self.proposals.read().await;
        proposals
            .values()
            .find(|p| p.vault_id() == vault_id && p.tx().txid() == txid)
            .map(|p| p.description().clone())
    }

    pub async fn txs_descriptions(&self, vault_id: VaultIdentifier) -> HashMap<Txid, String> {
        let mut map = HashMap::new();
        let proposals = self.proposals.read().await;
        for proposal in proposals.values().filter(|p| p.vault_id() == vault_id) {
            if proposal.is_finalized() {
                if let HashMapEntry::Vacant(e) = map.entry(proposal.tx().txid()) {
                    e.insert(proposal.description().clone());
                }
            }
        }
        map
    }

    pub async fn save_signer(&self, signer_id: SignerIdentifier, signer: Signer) {
        let mut signers = self.signers.write().await;
        signers.insert(signer_id, signer);
    }

    pub async fn delete_signer(&self, signer_id: &SignerIdentifier) -> bool {
        let mut signers = self.signers.write().await;
        signers.remove(signer_id).is_some()
    }

    /// Get signers
    pub async fn signers(&self) -> HashMap<SignerIdentifier, Signer> {
        self.signers
            .read()
            .await
            .iter()
            .map(|(id, s)| (*id, s.clone()))
            .collect()
    }

    /// Get [`Signer`]
    pub async fn signer(&self, signer_id: &SignerIdentifier) -> Result<Signer, Error> {
        let signers = self.signers.read().await;
        signers.get(signer_id).cloned().ok_or(Error::NotFound)
    }

    pub async fn signer_exists(&self, signer_id: &SignerIdentifier) -> bool {
        let signers = self.signers.read().await;
        signers.contains_key(signer_id)
    }

    pub async fn save_my_shared_signer(
        &self,
        signer_id: SignerIdentifier,
        shared_signer_id: EventId,
        public_key: PublicKey,
    ) {
        let mut my_shared_signers = self.my_shared_signers.write().await;
        my_shared_signers.insert(signer_id, (shared_signer_id, public_key));
    }

    /// Delete shared signer from both `shared_signers` and `my_shared_signers` collections
    pub async fn delete_shared_signer(&self, shared_signer_id: &EventId) -> bool {
        let mut my_shared_signers = self.my_shared_signers.write().await;
        my_shared_signers.retain(|_, (id, ..)| id == shared_signer_id);
        let mut shared_signers = self.shared_signers.write().await;
        shared_signers.remove(shared_signer_id).is_some()
    }

    pub async fn my_shared_signers(&self) -> HashMap<EventId, PublicKey> {
        self.my_shared_signers
            .read()
            .await
            .iter()
            .map(|(_, (id, p))| (*id, *p))
            .collect()
    }

    pub async fn shared_signers(&self) -> HashMap<EventId, SharedSigner> {
        self.shared_signers
            .read()
            .await
            .iter()
            .map(|(id, internal)| (*id, internal.clone()))
            .collect()
    }

    pub async fn my_shared_signer_already_shared(
        &self,
        signer_id: &SignerIdentifier,
        public_key: PublicKey,
    ) -> bool {
        if let Some((_, pk)) = self.my_shared_signers.read().await.get(signer_id) {
            if *pk == public_key {
                return true;
            }
        }
        false
    }

    pub async fn get_public_key_for_my_shared_signer(
        &self,
        shared_signer_id: EventId,
    ) -> Result<PublicKey, Error> {
        self.my_shared_signers
            .read()
            .await
            .values()
            .filter_map(|(id, pk)| {
                if *id == shared_signer_id {
                    Some(*pk)
                } else {
                    None
                }
            })
            .take(1)
            .next()
            .ok_or(Error::NotFound)
    }

    pub async fn get_my_shared_signers_by_signer_id(
        &self,
        signer_id: &SignerIdentifier,
    ) -> BTreeMap<EventId, PublicKey> {
        self.my_shared_signers
            .read()
            .await
            .iter()
            .filter(|(id, _)| *id == signer_id)
            .map(|(_, (k, v))| (*k, *v))
            .collect()
    }

    pub async fn get_shared_signers_public_keys(&self) -> HashSet<PublicKey> {
        self.shared_signers
            .read()
            .await
            .values()
            .map(|i| i.owner())
            .copied()
            .collect()
    }

    pub async fn get_shared_signers_by_public_key(
        &self,
        public_key: PublicKey,
    ) -> Vec<(EventId, SharedSigner)> {
        self.shared_signers
            .read()
            .await
            .iter()
            .filter(|(_, s)| *s.owner() == public_key)
            .map(|(id, s)| (*id, s.clone()))
            .collect()
    }

    pub async fn save_label<S>(&self, identifier: S, vault_id: VaultIdentifier, label: Label)
    where
        S: Into<String>,
    {
        let mut labels = self.labels.write().await;
        labels.insert(identifier.into(), InternalLabel { vault_id, label });
    }

    pub async fn get_addresses_labels(
        &self,
        vault_id: VaultIdentifier,
    ) -> HashMap<ScriptBuf, Label> {
        self.labels
            .read()
            .await
            .values()
            .filter(|i| i.label.kind() == LabelKind::Address && i.vault_id == vault_id)
            .filter_map(|i| {
                if let LabelData::Address(address) = i.label.data() {
                    Some((address.payload.script_pubkey(), i.label.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub async fn get_utxos_labels(&self, vault_id: VaultIdentifier) -> HashMap<OutPoint, Label> {
        self.labels
            .read()
            .await
            .values()
            .filter(|i| i.label.kind() == LabelKind::Utxo && i.vault_id == vault_id)
            .filter_map(|i| {
                if let LabelData::Utxo(utxo) = i.label.data() {
                    Some((utxo, i.label.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    pub async fn get_label_by_identifier<S>(&self, identifier: S) -> Result<Label, Error>
    where
        S: AsRef<str>,
    {
        self.labels
            .read()
            .await
            .get(identifier.as_ref())
            .map(|i| i.label.clone())
            .ok_or(Error::NotFound)
    }

    pub async fn freeze_utxos<I>(&self, vault_id: VaultIdentifier, utxos: I)
    where
        I: IntoIterator<Item = OutPoint> + Clone,
    {
        let mut frozed_utxos = self.frozed_utxos.write().await;
        frozed_utxos
            .entry(vault_id)
            .and_modify(|set| {
                set.extend(utxos.clone());
            })
            .or_default()
            .extend(utxos);
    }

    pub async fn get_frozen_utxos(&self, vault_id: &VaultIdentifier) -> HashSet<OutPoint> {
        self.frozed_utxos
            .read()
            .await
            .get(vault_id)
            .cloned()
            .unwrap_or_default()
    }

    pub async fn verified_key_agents(&self) -> VerifiedKeyAgents {
        self.verified_key_agents.read().await.clone()
    }
}
