// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::database::Order;
use nostr_sdk::{Event, EventBuilder, Filter, Keys, Kind, Tag};
use smartvaults_protocol::v2::constants::PROPOSAL_KIND_V2;
use smartvaults_protocol::v2::{self, Proposal, ProposalIdentifier, Vault, VaultIdentifier};

use super::{Error, SmartVaults};
use crate::storage::InternalVault;
use crate::types::GetProposal;

impl SmartVaults {
    pub(super) async fn internal_save_proposal(
        &self,
        proposal_id: &ProposalIdentifier,
        vault: &Vault,
        proposal: &Proposal,
    ) -> Result<(), Error> {
        // Compose and publish event
        let event = v2::proposal::build_event(vault, proposal)?;
        self.client.send_event(event).await?;

        // Index proposal
        self.storage
            .save_proposal(*proposal_id, proposal.clone())
            .await;

        Ok(())
    }

    async fn internal_get_proposal(
        &self,
        proposal_id: ProposalIdentifier,
        proposal: Proposal,
    ) -> GetProposal {
        if proposal.is_finalized() {
            GetProposal {
                signed: true,
                proposal,
            }
        } else {
            let approvals = self
                .storage
                .approvals()
                .await
                .into_iter()
                .filter(|(_, i)| i.approval.proposal_id() == proposal_id)
                .map(|(_, i)| i.approval);

            GetProposal {
                signed: proposal.try_finalize(approvals).is_ok(),
                proposal,
            }
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_proposal_by_id(
        &self,
        proposal_id: &ProposalIdentifier,
    ) -> Result<GetProposal, Error> {
        let proposal = self.storage.proposal(proposal_id).await?;
        Ok(self.internal_get_proposal(*proposal_id, proposal).await)
    }

    pub async fn delete_proposal_by_id(
        &self,
        proposal_id: &ProposalIdentifier,
    ) -> Result<(), Error> {
        // Get the proposal
        let proposal: Proposal = self.storage.proposal(proposal_id).await?;

        // Get Vault for shared key
        let InternalVault { vault, .. } = self.storage.vault(&proposal.vault_id()).await?;
        let shared_key: Keys = Keys::new(vault.shared_key().clone());

        let filter: Filter = Filter::new()
            .kind(PROPOSAL_KIND_V2)
            .author(shared_key.public_key())
            .identifier(proposal_id.to_string())
            .limit(1);
        let res: Vec<Event> = self
            .client
            .database()
            .query(vec![filter], Order::Desc)
            .await?;
        let proposal_event: &Event = res.first().ok_or(Error::NotFound)?;

        if proposal_event.author() == shared_key.public_key() {
            let event: Event =
                EventBuilder::new(Kind::EventDeletion, "", [Tag::event(proposal_event.id)])
                    .to_event(&shared_key)?;
            self.client.send_event(event).await?;

            self.storage.delete_proposal(proposal_id).await;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn proposals(&self) -> Result<Vec<GetProposal>, Error> {
        let proposals = self.storage.proposals().await;
        let mut list = Vec::with_capacity(proposals.len());
        for (proposal_id, proposal) in proposals.into_iter() {
            let proposal: GetProposal = self.internal_get_proposal(proposal_id, proposal).await;
            list.push(proposal);
        }
        list.sort();
        Ok(list)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn proposals_by_vault_id(
        &self,
        vault_id: VaultIdentifier,
    ) -> Result<Vec<GetProposal>, Error> {
        let proposals = self.storage.proposals().await;
        let mut list = Vec::with_capacity(proposals.len());
        for (proposal_id, proposal) in proposals
            .into_iter()
            .filter(|(_, p)| p.vault_id() == vault_id)
        {
            let proposal: GetProposal = self.internal_get_proposal(proposal_id, proposal).await;
            list.push(proposal);
        }
        list.sort();
        Ok(list)
    }

    /// Edit [Proposal] description
    pub async fn edit_proposal_description<S>(
        &self,
        proposal_id: &ProposalIdentifier,
        description: S,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let mut proposal = self.storage.proposal(proposal_id).await?;
        let InternalVault { vault, .. } = self.storage.vault(&proposal.vault_id()).await?;

        proposal.change_description(description);

        self.internal_save_proposal(proposal_id, &vault, &proposal)
            .await
    }
}
