// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap, HashSet};

use nostr_sdk::database::{NostrDatabaseExt, Order};
use nostr_sdk::nips::nip01::Coordinate;
use nostr_sdk::{Event, EventBuilder, EventId, Filter, Keys, Profile, PublicKey};
use smartvaults_core::bitcoin::address::NetworkChecked;
use smartvaults_core::bitcoin::{Address, Amount, OutPoint};
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::{Destination, FeeRate, Recipient, SpendingProposal};
use smartvaults_protocol::v1::constants::{KEY_AGENT_SIGNALING, KEY_AGENT_SIGNER_OFFERING_KIND};
use smartvaults_protocol::v1::{Serde, SignerOffering, SmartVaultsEventBuilder, VerifiedKeyAgents};
use smartvaults_protocol::v2::{self, PendingProposal, Period, Proposal, Signer, VaultIdentifier};

use super::{Error, SmartVaults};
use crate::storage::InternalVault;
use crate::types::{GetProposal, GetSignerOffering, KeyAgent};

impl SmartVaults {
    /// Announce as Key Agent
    pub async fn announce_key_agent(&self) -> Result<EventId, Error> {
        // Get keys
        let keys: &Keys = self.keys();

        // Compose event
        let event: Event = EventBuilder::key_agent_signaling(keys, self.network)?;

        // Publish event
        Ok(self.client.send_event(event).await?)
    }

    /// De-announce Key Agent (delete Key Agent signaling event)
    pub async fn deannounce_key_agent(&self) -> Result<(), Error> {
        let keys: &Keys = self.keys();
        let coordinate: Coordinate = Coordinate::new(KEY_AGENT_SIGNALING, keys.public_key())
            .identifier(self.network.magic().to_string());
        let event: EventBuilder = EventBuilder::delete([coordinate]);
        self.client.send_event_builder(event).await?;
        tracing::info!("Deleted Key Agent signaling for {}", keys.public_key());
        Ok(())
    }

    /// Create/Edit signer offering
    pub async fn signer_offering(
        &self,
        signer: &Signer,
        offering: SignerOffering,
    ) -> Result<EventId, Error> {
        // Get keys
        let keys: &Keys = self.keys();

        // Check if exists key agent signaling event
        let filter = Filter::new()
            .identifier(self.network.magic().to_string())
            .kind(KEY_AGENT_SIGNALING)
            .author(keys.public_key())
            .limit(1);
        let res = self
            .client
            .database()
            .event_ids_by_filters(vec![filter], Order::Desc)
            .await?;

        if res.is_empty() {
            self.announce_key_agent().await?;
        } else {
            tracing::debug!("Key agent already announced");
        }

        // Compose and publish event
        let event: Event = v2::key_agent::build_event(keys, signer, &offering)?;
        Ok(self.client.send_event(event).await?)
    }

    /// Delete signer offering for [`Signer`]
    pub async fn delete_signer_offering(&self, signer: &Signer) -> Result<(), Error> {
        // Get keys
        let keys: &Keys = self.keys();

        // Delete signer offering
        let coordinate: Coordinate =
            Coordinate::new(KEY_AGENT_SIGNER_OFFERING_KIND, keys.public_key())
                .identifier(signer.nostr_public_identifier().to_string());
        let event: EventBuilder = EventBuilder::delete([coordinate]);
        self.client.send_event_builder(event).await?;

        // Check if I have other signer offerings. If not, delete key agent signaling
        let filter = Filter::new()
            .kind(KEY_AGENT_SIGNER_OFFERING_KIND)
            .author(keys.public_key())
            .limit(1);
        let count: usize = self.client.database().count(vec![filter]).await?;

        if count == 0 {
            self.deannounce_key_agent().await?;
        } else {
            tracing::debug!(
                "User have some active signer offering, skipping key agent de-signaling"
            );
        }

        Ok(())
    }

    /// Get my signer offerings
    pub async fn my_signer_offerings(&self) -> Result<Vec<GetSignerOffering>, Error> {
        // Get keys
        let keys = self.keys();

        // Get signers
        let signers: HashMap<String, Signer> = self
            .signers()
            .await
            .into_iter()
            .map(|signer| (signer.nostr_public_identifier().to_string(), signer))
            .collect();

        // Get signer offering events by author
        let filter = Filter::new()
            .kind(KEY_AGENT_SIGNER_OFFERING_KIND)
            .author(keys.public_key());
        Ok(self
            .client
            .database()
            .query(vec![filter], Order::Desc)
            .await?
            .into_iter()
            .filter_map(|event| {
                let identifier: &str = event.identifier()?;
                let signer: Signer = signers.get(identifier)?.clone();
                let offering: SignerOffering = SignerOffering::from_json(event.content()).ok()?;
                if offering.network == self.network {
                    Some(GetSignerOffering {
                        id: event.id,
                        signer,
                        offering,
                    })
                } else {
                    None
                }
            })
            .collect())
    }

    /// Get Key Agents
    pub async fn key_agents(&self) -> Result<Vec<KeyAgent>, Error> {
        // Get contacts to check if key agent it's already added
        let keys = self.keys();
        let contacts = self
            .client
            .database()
            .contacts_public_keys(keys.public_key())
            .await?;

        // Get verified key agents
        let verified_key_agents: VerifiedKeyAgents = self.storage.verified_key_agents().await;

        // Get key agents and signer offerings
        let filters: Vec<Filter> = vec![
            Filter::new()
                .kind(KEY_AGENT_SIGNALING)
                .identifier(self.network.magic().to_string()),
            Filter::new().kind(KEY_AGENT_SIGNER_OFFERING_KIND),
        ];
        let mut key_agents: HashMap<PublicKey, HashSet<SignerOffering>> = HashMap::new();

        for event in self
            .client
            .database()
            .query(filters, Order::Desc)
            .await?
            .into_iter()
        {
            if event.kind == KEY_AGENT_SIGNALING {
                key_agents.entry(event.author()).or_default();
            } else if event.kind == KEY_AGENT_SIGNER_OFFERING_KIND {
                if let Ok(signer_offering) = SignerOffering::from_json(event.content()) {
                    // Check network
                    if signer_offering.network == self.network {
                        key_agents
                            .entry(event.author())
                            .and_modify(|set| {
                                set.insert(signer_offering);
                            })
                            .or_insert_with(|| {
                                let mut set = HashSet::new();
                                set.insert(signer_offering);
                                set
                            });
                    }
                }
            }
        }

        let mut list = Vec::with_capacity(key_agents.len());
        for (public_key, set) in key_agents.into_iter() {
            let metadata = self.get_public_key_metadata(public_key).await?;
            list.push(KeyAgent {
                user: Profile::new(public_key, metadata),
                list: set,
                verified: verified_key_agents.is_verified(&public_key),
                is_contact: contacts.contains(&public_key),
            })
        }
        list.sort();
        Ok(list)
    }

    /// Request signers to Key Agent
    pub async fn request_signers_to_key_agent(&self, key_agent: PublicKey) -> Result<(), Error> {
        self.add_contact(key_agent).await?;
        Ok(())
    }

    pub async fn key_agent_payment<S>(
        &self,
        vault_id: &VaultIdentifier,
        address: Address<NetworkChecked>,
        amount: Amount,
        description: S,
        signer_descriptor: Descriptor<String>,
        period: Period,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
        skip_frozen_utxos: bool,
    ) -> Result<GetProposal, Error>
    where
        S: Into<String>,
    {
        let recipient = Recipient { address, amount };
        let spending_proposal: SpendingProposal = self
            .internal_spend(
                vault_id,
                &Destination::Single(recipient.clone()),
                fee_rate,
                utxos,
                policy_path,
                skip_frozen_utxos,
            )
            .await?;
        let pending = PendingProposal::KeyAgentPayment {
            descriptor: spending_proposal.descriptor,
            signer_descriptor,
            recipient,
            period,
            description: description.into(),
            psbt: spending_proposal.psbt,
        };
        let proposal = Proposal::pending(*vault_id, pending, self.network);

        // Get vault
        let InternalVault { vault, .. } = self.storage.vault(vault_id).await?;

        // Compose the event
        let event: Event = v2::proposal::build_event(&vault, &proposal)?;
        self.client.send_event(event).await?;

        // Index proposal
        self.storage
            .save_proposal(proposal.compute_id(), proposal.clone())
            .await;

        Ok(GetProposal {
            proposal,
            signed: false,
        })
    }
}
