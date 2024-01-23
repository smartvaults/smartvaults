// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::cmp::Ordering;
use std::collections::hash_map::Entry as HashMapEntry;
use std::collections::{HashMap, HashSet};
use std::str::FromStr;
use std::sync::Arc;

use nostr_sdk::prelude::*;
use smartvaults_core::bitcoin::{Network, OutPoint, ScriptBuf, Txid};
use smartvaults_protocol::v1::constants::{
    KEY_AGENT_VERIFIED, SHARED_SIGNERS_KIND, SMARTVAULTS_MAINNET_PUBLIC_KEY,
    SMARTVAULTS_TESTNET_PUBLIC_KEY,
};
use smartvaults_protocol::v1::{Label, LabelData, LabelKind, VerifiedKeyAgents};
use smartvaults_protocol::v2::constants::{
    APPROVAL_KIND_V2, PROPOSAL_KIND_V2, SIGNER_KIND_V2, VAULT_KIND_V2, VAULT_METADATA_KIND_V2,
};
use smartvaults_protocol::v2::{
    Approval, NostrPublicIdentifier, Proposal, ProposalIdentifier, ProtocolEncryption,
    SharedSigner, Signer, SignerIdentifier, Vault, VaultIdentifier, VaultMetadata,
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
    network: Network,
    vaults_ids: Arc<RwLock<HashMap<EventId, VaultIdentifier>>>,
    vaults_keys: Arc<RwLock<HashMap<PublicKey, Keys>>>,
    vaults: Arc<RwLock<HashMap<VaultIdentifier, InternalVault>>>,
    proposals_ids: Arc<RwLock<HashMap<EventId, ProposalIdentifier>>>,
    proposals: Arc<RwLock<HashMap<ProposalIdentifier, Proposal>>>,
    approvals: Arc<RwLock<HashMap<EventId, InternalApproval>>>,
    signers: Arc<RwLock<HashMap<SignerIdentifier, Signer>>>,
    shared_signers: Arc<RwLock<HashMap<NostrPublicIdentifier, SharedSigner>>>,
    labels: Arc<RwLock<HashMap<String, InternalLabel>>>,
    frozed_utxos: Arc<RwLock<HashMap<VaultIdentifier, HashSet<OutPoint>>>>,
    verified_key_agents: Arc<RwLock<VerifiedKeyAgents>>,
}

impl SmartVaultsStorage {
    /// Build storage from Nostr Database
    ///
    /// ### Steps
    /// 1. Get all `vaults`
    /// 2. Get all events authored by vaults shared keys
    /// 3. Get other events
    #[tracing::instrument(skip_all)]
    pub async fn build(
        keys: Keys,
        database: Arc<DynNostrDatabase>,
        network: Network,
    ) -> Result<Self, Error> {
        let this: Self = Self {
            keys,
            database,
            network,
            vaults_ids: Arc::new(RwLock::new(HashMap::new())),
            vaults_keys: Arc::new(RwLock::new(HashMap::new())),
            vaults: Arc::new(RwLock::new(HashMap::new())),
            proposals_ids: Arc::new(RwLock::new(HashMap::new())),
            proposals: Arc::new(RwLock::new(HashMap::new())),
            approvals: Arc::new(RwLock::new(HashMap::new())),
            signers: Arc::new(RwLock::new(HashMap::new())),
            shared_signers: Arc::new(RwLock::new(HashMap::new())),
            labels: Arc::new(RwLock::new(HashMap::new())),
            frozed_utxos: Arc::new(RwLock::new(HashMap::new())),
            verified_key_agents: Arc::new(RwLock::new(VerifiedKeyAgents::empty(network))),
        };

        // Step 1: get all vaults
        let step1: Filter = Filter::new()
            .author(this.keys.public_key())
            .kind(VAULT_KIND_V2);
        for event in this
            .database
            .query(vec![step1], Order::Asc)
            .await?
            .into_iter()
        {
            if let Err(e) = this.handle_event(&event).await {
                tracing::error!("Impossible to handle vault event: {e}");
            }
        }

        // Step 2: get events authored by vaults
        let vault_keys = this.vaults_keys.read().await;
        let vaults_pubkeys = vault_keys.clone().into_keys();
        drop(vault_keys);
        let step2: Filter = Filter::new().authors(vaults_pubkeys);
        for event in this
            .database
            .query(vec![step2], Order::Asc)
            .await?
            .into_iter()
        {
            if let Err(e) = this.handle_event(&event).await {
                tracing::error!("Impossible to handle vault event: {e}");
            }
        }

        // Step 3: get other events
        let smartvaults: Filter = Filter::new()
            .author(match network {
                Network::Bitcoin => *SMARTVAULTS_MAINNET_PUBLIC_KEY,
                _ => *SMARTVAULTS_TESTNET_PUBLIC_KEY,
            })
            .kind(KEY_AGENT_VERIFIED);
        for event in this
            .database
            .query(vec![smartvaults], Order::Asc)
            .await?
            .into_iter()
        {
            if let Err(e) = this.handle_event(&event).await {
                tracing::error!("Impossible to handle event: {e}");
            }
        }

        Ok(this)
    }

    pub(crate) async fn handle_event(&self, event: &Event) -> Result<Option<EventHandled>, Error> {
        if event.kind == VAULT_KIND_V2 {
            // TODO: remove vaults_ids?
            let mut vaults_ids = self.vaults_ids.write().await;
            let mut vaults_keys = self.vaults_keys.write().await;
            let mut vaults = self.vaults.write().await;
            if let HashMapEntry::Vacant(e) = vaults_ids.entry(event.id) {
                let vault: Vault = Vault::decrypt_with_keys(&self.keys, &event.content)?;
                let vault_id = vault.id();
                let keys = Keys::new(vault.shared_key());
                let internal = InternalVault {
                    vault,
                    metadata: VaultMetadata::new(vault_id, self.network)
                };
                e.insert(vault_id);
                vaults_keys.insert(keys.public_key(), keys);
                vaults.insert(vault_id, internal);
                return Ok(Some(EventHandled::Vault(vault_id)));
            }
        } else if event.kind == VAULT_METADATA_KIND_V2 {
            let vaults_keys = self.vaults_keys.read().await;
            let mut vaults = self.vaults.write().await;
            if let Some(shared_key) = vaults_keys.get(&event.pubkey) {
                let metadata: VaultMetadata = VaultMetadata::decrypt_with_keys(shared_key, &event.content)?;
                let vault_id = metadata.vault_id();
                if let Some(vault) = vaults.get_mut(&vault_id) {
                    vault.metadata = metadata;
                    return Ok(Some(EventHandled::VaultMetadata(vault_id)));
                }
            }
        } else if event.kind == PROPOSAL_KIND_V2 {
            let vaults_keys = self.vaults_keys.read().await;
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
                }
            }
        } else if event.kind == APPROVAL_KIND_V2 {
            let vaults_keys = self.vaults_keys.read().await;
            let mut approvals = self.approvals.write().await;
            if let HashMapEntry::Vacant(e) = approvals.entry(event.id) {
                // Get public key of the shared key used for encrypt the approval
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
                // TODO: add private encrypted list of signers shared with who and when
            } else {
                let mut shared_signers = self.shared_signers.write().await;
                let shared_signer_id = event.identifier().ok_or(Error::SharedSignerIdNotFound)?;
                let id = NostrPublicIdentifier::from_str(shared_signer_id)?;
                let shared_signer: SharedSigner = SharedSigner::decrypt_with_keys(self.keys, event.content())?;
                shared_signers.entry(id).and_modify(|s| {
                    // Update only if newer timestamp
                    if s.timestamp() < shared_signer.timestamp() {
                        *s = shared_signer.clone();
                    }
                }).or_insert(shared_signer);
                return Ok(Some(EventHandled::SharedSigner(event.id)));
            }
        } /* else if event.kind == LABELS_KIND {
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
        } */ else if event.kind == Kind::EventDeletion {
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

    /// Delete event without know the kind
    pub async fn delete_event(&self, event_id: &EventId) {
        if self.delete_approval(event_id).await {
            return;
        }

        // TODO: delete proposal, signer, ...
    }

    /// Save [Vault] without [VaultMetadata]
    pub async fn save_vault(&self, vault_id: VaultIdentifier, vault: Vault) {
        let mut vaults = self.vaults.write().await;
        vaults.insert(
            vault_id,
            InternalVault {
                vault,
                metadata: VaultMetadata::new(vault_id, self.network),
            },
        );
    }

    pub async fn delete_vault(&self, vault_id: &VaultIdentifier) -> bool {
        let mut vaults = self.vaults.write().await;

        // Delete vault
        match vaults.remove(vault_id) {
            Some(internal) => {
                // Delete vault key
                let mut vaults_keys = self.vaults_keys.write().await;
                let keys = Keys::new(internal.vault.shared_key());
                vaults_keys.remove(&keys.public_key());
                drop(vaults_keys);

                // Delete other things related to vault?

                true
            }
            None => false,
        }
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

    /// Delete shared signer from both `shared_signers` and `my_shared_signers` collections
    pub async fn delete_shared_signer(&self, shared_signer_id: &NostrPublicIdentifier) -> bool {
        let mut shared_signers = self.shared_signers.write().await;
        shared_signers.remove(shared_signer_id).is_some()
    }

    pub async fn shared_signers(&self) -> HashMap<NostrPublicIdentifier, SharedSigner> {
        self.shared_signers
            .read()
            .await
            .iter()
            .map(|(id, shared_signer)| (*id, shared_signer.clone()))
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
    ) -> Vec<(NostrPublicIdentifier, SharedSigner)> {
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
