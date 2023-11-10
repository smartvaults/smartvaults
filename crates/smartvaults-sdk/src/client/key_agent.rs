// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashSet};

use nostr_sdk::secp256k1::XOnlyPublicKey;
use nostr_sdk::{Event, EventBuilder, EventId, Keys};
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::{Address, OutPoint};
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::proposal::Period;
use smartvaults_core::{Amount, FeeRate, Proposal, Signer};
use smartvaults_protocol::v1::{SignerOffering, SmartVaultsEventBuilder};
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
        self.send_event(event).await
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
        let id: EventId = self.send_event(event).await?;

        // Update key agents list // TODO: save in local DB?
        let mut key_agents = self.key_agents.write().await;
        key_agents
            .entry(keys.public_key())
            .and_modify(|set| {
                set.insert(offering.clone());
            })
            .or_insert_with(|| {
                let mut set = HashSet::new();
                set.insert(offering);
                set
            });

        Ok(id)
    }

    /// Get Key Agents
    pub async fn key_agents(&self) -> Result<Vec<KeyAgent>, Error> {
        let contacts = self.db.get_contacts_public_keys().await?;
        let verified_key_agents = self.verified_key_agents.read().await;
        let agents = self.key_agents.read().await;
        let mut list = Vec::with_capacity(agents.len());
        for (public_key, set) in agents.clone().into_iter() {
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
