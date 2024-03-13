// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap, HashSet};

use nostr_sdk::database::{NostrDatabaseExt, Order};
use nostr_sdk::nips::nip01::Coordinate;
use nostr_sdk::{Event, EventBuilder, EventId, Filter, Keys, Profile, PublicKey};
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::{Address, OutPoint};
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::proposal::Period;
use smartvaults_core::{Amount, FeeRate, Proposal, Signer};
use smartvaults_protocol::v1::constants::{KEY_AGENT_SIGNALING, KEY_AGENT_SIGNER_OFFERING_KIND};
use smartvaults_protocol::v1::{Serde, SignerOffering, SmartVaultsEventBuilder, VerifiedKeyAgents};

use super::{Error, SmartVaults};
use crate::types::{GetProposal, GetSigner, GetSignerOffering, KeyAgent};

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

        // Compose event
        let event: Event = EventBuilder::signer_offering(keys, signer, &offering, self.network)?;

        // Publish event
        Ok(self.client.send_event(event).await?)
    }

    /// Delete signer offering for [`Signer`]
    pub async fn delete_signer_offering(&self, signer: &Signer) -> Result<(), Error> {
        // Get keys
        let keys: &Keys = self.keys();

        // Delete signer offering
        let coordinate: Coordinate =
            Coordinate::new(KEY_AGENT_SIGNER_OFFERING_KIND, keys.public_key())
                .identifier(signer.generate_identifier(self.network));
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
        let signers: HashMap<String, GetSigner> = self
            .get_signers()
            .await
            .into_iter()
            .map(|signer| {
                let identifier: String = signer.generate_identifier(self.network);
                (identifier, signer)
            })
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
                let signer: GetSigner = signers.get(identifier)?.clone();
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
        policy_id: EventId,
        address: Address<NetworkUnchecked>,
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
        let mut prop: GetProposal = self
            .spend(
                policy_id,
                address,
                amount,
                description,
                fee_rate,
                utxos,
                policy_path.clone(),
                skip_frozen_utxos,
            )
            .await?;
        if let Proposal::Spending {
            descriptor,
            amount,
            description,
            psbt,
            ..
        } = prop.proposal
        {
            prop.proposal = Proposal::KeyAgentPayment {
                descriptor,
                signer_descriptor,
                amount,
                description,
                period,
                psbt,
                policy_path,
            };
            Ok(prop)
        } else {
            Err(Error::UnexpectedProposal)
        }
    }
}
