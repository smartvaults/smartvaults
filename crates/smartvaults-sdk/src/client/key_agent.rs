// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap, HashSet};

use nostr_sdk::database::NostrDatabaseExt;
use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{Event, EventBuilder, EventId, Filter, Keys};
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::{Address, OutPoint};
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::proposal::Period;
use smartvaults_core::{Amount, FeeRate, Proposal, Signer};
use smartvaults_protocol::v1::constants::KEY_AGENT_SIGNER_OFFERING_KIND;
use smartvaults_protocol::v1::{Serde, SignerOffering, SmartVaultsEventBuilder};
use smartvaults_sdk_sqlite::model::GetProposal;

use super::{Error, SmartVaults};
use crate::types::{KeyAgent, User};

impl SmartVaults {
    /// Announce as Key Agent
    pub async fn announce_key_agent(&self) -> Result<EventId, Error> {
        // Get keys
        let keys: Keys = self.keys().await;

        // Compose event
        let event: Event = EventBuilder::key_agent_signaling(&keys, self.network)?;

        // Publish event
        Ok(self.client.send_event(event).await?)
    }

    pub async fn signer_offering(
        &self,
        signer: &Signer,
        offering: SignerOffering,
    ) -> Result<EventId, Error> {
        // Get keys
        let keys: Keys = self.keys().await;

        // Compose event
        let event: Event = EventBuilder::signer_offering(&keys, signer, &offering, self.network)?;

        // Publish event
        Ok(self.client.send_event(event).await?)
    }

    /// Get Key Agents
    pub async fn key_agents(&self) -> Result<Vec<KeyAgent>, Error> {
        // Get contacts to check if key agent it's already added
        let keys = self.client.keys().await;
        let contacts = self
            .client
            .database()
            .contacts_public_keys(keys.public_key())
            .await?;

        // Get verified key agents
        let verified_key_agents = self.verified_key_agents.read().await;

        // Get key agents and signer offerings
        let filter = Filter::new().kind(KEY_AGENT_SIGNER_OFFERING_KIND);
        let mut key_agents: HashMap<XOnlyPublicKey, HashSet<SignerOffering>> = HashMap::new();

        for event in self
            .client
            .database()
            .query(vec![filter])
            .await?
            .into_iter()
        {
            if let Ok(signer_offering) = SignerOffering::from_json(event.content) {
                // Check network
                if signer_offering.network == self.network {
                    key_agents
                        .entry(event.pubkey)
                        .and_modify(|set| {
                            set.insert(signer_offering.clone());
                        })
                        .or_insert_with(|| {
                            let mut set = HashSet::new();
                            set.insert(signer_offering);
                            set
                        });
                }
            }
        }

        let mut list = Vec::with_capacity(key_agents.len());
        for (public_key, set) in key_agents.into_iter() {
            let metadata = self.get_public_key_metadata(public_key).await?;
            list.push(KeyAgent {
                user: User::new(public_key, metadata),
                list: set,
                verified: verified_key_agents.is_verified(&public_key),
                is_contact: contacts.contains(&public_key),
            })
        }
        Ok(list)
    }

    /// Request signers to Key Agent
    pub async fn request_signers_to_key_agent(
        &self,
        key_agent: XOnlyPublicKey,
    ) -> Result<(), Error> {
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
                policy_path,
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
            };
            Ok(prop)
        } else {
            Err(Error::UnexpectedProposal)
        }
    }
}
