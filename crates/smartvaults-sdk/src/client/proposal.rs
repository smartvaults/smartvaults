// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use nostr_sdk::database::Order;
use nostr_sdk::{Event, EventBuilder, Filter, Keys, Kind, Tag};
use smartvaults_protocol::v2::constants::PROPOSAL_KIND_V2;
use smartvaults_protocol::v2::{Proposal, ProposalIdentifier, VaultIdentifier};

use super::{Error, SmartVaults};
use crate::storage::InternalVault;
use crate::types::GetProposal;

impl SmartVaults {
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_proposal_by_id(
        &self,
        proposal_id: &ProposalIdentifier,
    ) -> Result<GetProposal, Error> {
        let proposal = self.storage.proposal(proposal_id).await?;
        let approvals = self
            .storage
            .approvals()
            .await
            .into_iter()
            .filter(|(_, i)| i.approval.proposal_id() == *proposal_id)
            .map(|(_, i)| i.approval);
        Ok(GetProposal {
            signed: proposal.try_finalize(approvals).is_ok(),
            proposal,
        })
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
            let approvals = self
                .storage
                .approvals()
                .await
                .into_values()
                .filter(|i| i.approval.proposal_id() == proposal_id)
                .map(|i| i.approval);
            list.push(GetProposal {
                signed: proposal.try_finalize(approvals).is_ok(),
                proposal,
            });
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
            let approvals = self
                .storage
                .approvals()
                .await
                .into_values()
                .filter(|i| i.approval.proposal_id() == proposal_id)
                .map(|i| i.approval);
            list.push(GetProposal {
                signed: proposal.try_finalize(approvals).is_ok(),
                proposal,
            });
        }
        list.sort();
        Ok(list)
    }
}
