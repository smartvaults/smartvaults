// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;

use coinstr_sdk::client;
use coinstr_sdk::core::bdk::blockchain::{Blockchain, ElectrumBlockchain};
use coinstr_sdk::core::bdk::electrum_client::Client as ElectrumClient;
use coinstr_sdk::core::bips::bip39::Mnemonic;
use coinstr_sdk::core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_sdk::core::bitcoin::{Address, Network, Txid, XOnlyPublicKey};
use coinstr_sdk::core::types::WordCount;
use coinstr_sdk::db::model::GetApprovedProposalResult;
use coinstr_sdk::nostr::prelude::FromPkStr;
use coinstr_sdk::nostr::{self, block_on, EventId, Keys};

use crate::error::Result;
use crate::{
    Amount, Approval, Balance, CompletedProposal, KeychainSeed, Metadata, Policy, Proposal, Relay,
    Signer, TransactionDetails, Utxo,
};

pub struct Coinstr {
    inner: client::Coinstr,
}

impl Coinstr {
    /// Open keychain
    pub fn open(
        base_path: String,
        name: String,
        password: String,
        network: Network,
    ) -> Result<Self> {
        Ok(Self {
            inner: client::Coinstr::open(base_path, name, || Ok(password), network)?,
        })
    }

    /// Generate keychain
    pub fn generate(
        base_path: String,
        name: String,
        password: String,
        word_count: WordCount,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        Ok(Self {
            inner: client::Coinstr::generate(
                base_path,
                name,
                || Ok(password),
                word_count,
                || Ok(passphrase),
                network,
            )?,
        })
    }

    /// Restore keychain
    pub fn restore(
        base_path: String,
        name: String,
        password: String,
        mnemonic: String,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        let mnemonic = Mnemonic::from_str(&mnemonic)?;
        Ok(Self {
            inner: client::Coinstr::restore(
                base_path,
                name,
                || Ok(password),
                || Ok(mnemonic),
                || Ok(passphrase),
                network,
            )?,
        })
    }

    /// Get keychain name
    pub fn name(&self) -> Option<String> {
        self.inner.name()
    }

    /// Save keychain
    pub fn save(&self) -> Result<()> {
        Ok(self.inner.save()?)
    }

    /// Check keychain password
    pub fn check_password(&self, password: String) -> bool {
        self.inner.check_password(password)
    }

    // TODO: add `rename` method

    // TODO: add `change_password` method

    /// Permanent delete the keychain
    pub fn wipe(&self, password: String) -> Result<()> {
        Ok(self.inner.wipe(password)?)
    }

    pub fn clear_cache(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.clear_cache().await?) })
    }

    pub fn seed(&self) -> Arc<KeychainSeed> {
        Arc::new(self.inner.keychain().seed().into())
    }

    pub fn keys(&self) -> Arc<crate::Keys> {
        Arc::new(self.inner.keys().into())
    }

    pub fn network(&self) -> Network {
        self.inner.network()
    }

    /// Add new relay
    pub fn add_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.add_relay(url, None).await?) })
    }

    /// Connect relays
    pub fn connect(&self) {
        block_on(async move {
            self.inner.connect().await;
        })
    }

    pub fn default_relays(&self) -> Vec<String> {
        self.inner.default_relays()
    }

    pub fn restore_relays(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.restore_relays().await?) })
    }

    pub fn remove_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.remove_relay(url).await?) })
    }

    pub fn relays(&self) -> Vec<Arc<Relay>> {
        block_on(async move {
            self.inner
                .relays()
                .await
                .into_values()
                .map(|relay| Arc::new(relay.into()))
                .collect()
        })
    }

    /// Shutdown client
    pub fn shutdown(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.clone().shutdown().await?) })
    }

    /// Set the electrum endpoint
    pub fn set_electrum_endpoint(&self, endpoint: String) -> Result<()> {
        Ok(self.inner.set_electrum_endpoint(endpoint)?)
    }

    /// Get the electrum endpoint
    pub fn electrum_endpoint(&self) -> Result<String> {
        Ok(self.inner.electrum_endpoint()?)
    }

    pub fn block_height(&self) -> u32 {
        self.inner.block_height()
    }

    pub fn set_metadata(&self, json: String) -> Result<()> {
        block_on(async move {
            let metadata = nostr::Metadata::from_json(json)?;
            Ok(self.inner.set_metadata(metadata).await?)
        })
    }

    pub fn get_profile(&self) -> Result<Arc<Metadata>> {
        Ok(Arc::new(self.inner.get_profile()?.into()))
    }

    pub fn get_contacts(&self) -> Result<HashMap<String, Arc<Metadata>>> {
        Ok(self
            .inner
            .get_contacts()?
            .into_iter()
            .map(|(pk, m)| (pk.to_string(), Arc::new(m.into())))
            .collect())
    }

    /// Add new contact
    pub fn add_contact(&self, public_key: String) -> Result<()> {
        block_on(async move {
            let keys: Keys = Keys::from_pk_str(&public_key)?;
            Ok(self.inner.add_contact(keys.public_key()).await?)
        })
    }

    /// Remove contact
    pub fn remove_contact(&self, public_key: String) -> Result<()> {
        block_on(async move {
            let keys: Keys = Keys::from_pk_str(&public_key)?;
            Ok(self.inner.remove_contact(keys.public_key()).await?)
        })
    }

    pub fn get_policy_by_id(&self, policy_id: String) -> Result<Arc<Policy>> {
        let policy_id = EventId::from_hex(policy_id)?;
        Ok(Arc::new(self.inner.get_policy_by_id(policy_id)?.into()))
    }

    pub fn get_proposal_by_id(&self, proposal_id: String) -> Result<Proposal> {
        let proposal_id = EventId::from_hex(proposal_id)?;
        Ok(self.inner.get_proposal_by_id(proposal_id)?.1.into())
    }

    pub fn get_completed_proposal_by_id(
        &self,
        completed_proposal_id: String,
    ) -> Result<CompletedProposal> {
        let completed_proposal_id = EventId::from_hex(completed_proposal_id)?;
        Ok(self
            .inner
            .get_completed_proposal_by_id(completed_proposal_id)?
            .1
            .into())
    }

    pub fn get_signer_by_id(&self, signer_id: String) -> Result<Arc<Signer>> {
        let signer_id = EventId::from_hex(signer_id)?;
        Ok(Arc::new(self.inner.get_signer_by_id(signer_id)?.into()))
    }

    pub fn delete_policy_by_id(&self, policy_id: String) -> Result<()> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            Ok(self.inner.delete_policy_by_id(policy_id).await?)
        })
    }

    pub fn delete_proposal_by_id(&self, proposal_id: String) -> Result<()> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            Ok(self.inner.delete_proposal_by_id(proposal_id).await?)
        })
    }

    pub fn delete_completed_proposal_by_id(&self, completed_proposal_id: String) -> Result<()> {
        block_on(async move {
            let completed_proposal_id = EventId::from_hex(completed_proposal_id)?;
            Ok(self
                .inner
                .delete_completed_proposal_by_id(completed_proposal_id)
                .await?)
        })
    }

    pub fn delete_signer_by_id(&self, signer_id: String) -> Result<()> {
        block_on(async move {
            let signer_id = EventId::from_hex(signer_id)?;
            Ok(self.inner.delete_signer_by_id(signer_id).await?)
        })
    }

    pub fn get_policies(&self) -> Result<HashMap<String, Arc<Policy>>> {
        let policies = self.inner.get_policies()?;
        Ok(policies
            .into_iter()
            .map(|(policy_id, res)| (policy_id.to_hex(), Arc::new(res.policy.into())))
            .collect())
    }

    // TODO: add `get_detailed_policies` method

    pub fn get_proposals(&self) -> Result<HashMap<String, Proposal>> {
        let proposals = self.inner.get_proposals()?;
        Ok(proposals
            .into_iter()
            .map(|(proposal_id, (_, proposal))| (proposal_id.to_hex(), proposal.into()))
            .collect())
    }

    pub fn is_proposal_signed(&self, proposal_id: String) -> Result<bool> {
        let proposal_id = EventId::from_hex(proposal_id)?;
        let proposal = self.inner.get_proposal_by_id(proposal_id)?.1;
        let approvals = self
            .inner
            .get_approvals_by_proposal_id(proposal_id)?
            .iter()
            .map(
                |(
                    _,
                    GetApprovedProposalResult {
                        approved_proposal, ..
                    },
                )| { approved_proposal.clone() },
            )
            .collect();
        Ok(proposal.finalize(approvals, self.inner.network()).is_ok())
    }

    pub fn get_approvals_by_proposal_id(
        &self,
        proposal_id: String,
    ) -> Result<HashMap<String, Arc<Approval>>> {
        let proposal_id = EventId::from_hex(proposal_id)?;
        Ok(self
            .inner
            .get_approvals_by_proposal_id(proposal_id)?
            .into_iter()
            .map(|(id, res)| (id.to_hex(), Arc::new(res.into())))
            .collect())
    }

    pub fn get_completed_proposals(&self) -> Result<HashMap<String, CompletedProposal>> {
        let completed_proposals = self.inner.get_completed_proposals()?;
        Ok(completed_proposals
            .into_iter()
            .map(|(proposal_id, (_, proposal))| (proposal_id.to_hex(), proposal.into()))
            .collect())
    }

    pub fn save_policy(
        &self,
        name: String,
        description: String,
        descriptor: String,
        public_keys: Vec<String>,
    ) -> Result<String> {
        block_on(async move {
            let mut nostr_pubkeys: Vec<XOnlyPublicKey> = Vec::new();
            for pk in public_keys.into_iter() {
                nostr_pubkeys.push(XOnlyPublicKey::from_str(&pk)?);
            }
            Ok(self
                .inner
                .save_policy(name, description, descriptor, nostr_pubkeys)
                .await?
                .to_hex())
        })
    }

    pub fn spend(
        &self,
        policy_id: String,
        to_address: String,
        amount: Arc<Amount>,
        description: String,
        target_blocks: u16,
    ) -> Result<String> {
        block_on(async move {
            let endpoint: String = self.inner.electrum_endpoint()?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            let fee_rate = blockchain.estimate_fee(target_blocks as usize)?;

            let policy_id = EventId::from_hex(policy_id)?;
            let to_address = Address::from_str(&to_address)?;
            let (proposal_id, ..) = self
                .inner
                .spend(policy_id, to_address, amount.inner(), description, fee_rate)
                .await?;
            Ok(proposal_id.to_hex())
        })
    }

    pub fn self_transfer(
        &self,
        from_policy_id: String,
        to_policy_id: String,
        amount: Arc<Amount>,
        target_blocks: u16,
    ) -> Result<String> {
        block_on(async move {
            let endpoint: String = self.inner.electrum_endpoint()?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            let fee_rate = blockchain.estimate_fee(target_blocks as usize)?;

            let from_policy_id = EventId::from_hex(from_policy_id)?;
            let to_policy_id = EventId::from_hex(to_policy_id)?;
            let (proposal_id, ..) = self
                .inner
                .self_transfer(from_policy_id, to_policy_id, amount.inner(), fee_rate)
                .await?;
            Ok(proposal_id.to_hex())
        })
    }

    pub fn approve(&self, proposal_id: String) -> Result<String> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            let (approval_id, ..) = self.inner.approve(proposal_id).await?;
            Ok(approval_id.to_hex())
        })
    }

    pub fn approve_with_signed_psbt(
        &self,
        proposal_id: String,
        signed_psbt: String,
    ) -> Result<String> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            let signed_psbt = PartiallySignedTransaction::from_str(&signed_psbt)?;
            let (approval_id, ..) = self
                .inner
                .approve_with_signed_psbt(proposal_id, signed_psbt)
                .await?;
            Ok(approval_id.to_hex())
        })
    }

    pub fn revoke_approval(&self, approval_id: String) -> Result<()> {
        block_on(async move {
            let approval_id = EventId::from_hex(approval_id)?;
            Ok(self.inner.revoke_approval(approval_id).await?)
        })
    }

    pub fn finalize(&self, proposal_id: String) -> Result<CompletedProposal> {
        block_on(async move {
            let proposal_id = EventId::from_hex(proposal_id)?;
            Ok(self.inner.finalize(proposal_id).await?.into())
        })
    }

    pub fn new_proof_proposal(&self, policy_id: String, message: String) -> Result<String> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            Ok(self
                .inner
                .new_proof_proposal(policy_id, message)
                .await?
                .0
                .to_hex())
        })
    }

    // TODO: add verify_proof

    // TODO: add verify_proof_by_id

    // TODO: add save_signer

    pub fn coinstr_signer_exists(&self) -> Result<bool> {
        Ok(self.inner.coinstr_signer_exists()?)
    }

    pub fn save_coinstr_signer(&self) -> Result<String> {
        block_on(async move { Ok(self.inner.save_coinstr_signer().await?.to_hex()) })
    }

    // TODO: add get_all_signers

    // TODO: add get_signers

    pub fn sync(&self) {
        self.inner.sync();
    }

    pub fn get_balance(&self, policy_id: String) -> Result<Option<Arc<Balance>>> {
        let policy_id = EventId::from_hex(policy_id)?;
        Ok(self
            .inner
            .get_balance(policy_id)
            .map(|b| Arc::new(b.into())))
    }

    pub fn get_txs(&self, policy_id: String) -> Result<Vec<Arc<TransactionDetails>>> {
        let policy_id = EventId::from_hex(policy_id)?;
        Ok(self
            .inner
            .get_txs(policy_id)
            .unwrap_or_default()
            .into_iter()
            .map(|tx| Arc::new(tx.into()))
            .collect())
    }

    pub fn get_utxos(&self, policy_id: String) -> Result<Vec<Arc<Utxo>>> {
        let policy_id = EventId::from_hex(policy_id)?;
        Ok(self
            .inner
            .get_utxos(policy_id)?
            .into_iter()
            .map(|u| Arc::new(u.into()))
            .collect())
    }

    pub fn get_total_balance(&self) -> Result<Arc<Balance>> {
        Ok(Arc::new(self.inner.get_total_balance()?.into()))
    }

    pub fn get_all_txs(&self) -> Result<Vec<Arc<TransactionDetails>>> {
        Ok(self
            .inner
            .get_all_transactions()?
            .into_iter()
            .map(|(tx, ..)| Arc::new(tx.into()))
            .collect())
    }

    pub fn get_tx(&self, txid: String) -> Result<Option<Arc<TransactionDetails>>> {
        let txid = Txid::from_str(&txid)?;
        Ok(self.inner.get_tx(txid).map(|(tx, ..)| Arc::new(tx.into())))
    }

    pub fn get_last_unused_address(&self, policy_id: String) -> Result<Option<String>> {
        let policy_id = EventId::from_hex(policy_id)?;
        Ok(self
            .inner
            .get_last_unused_address(policy_id)
            .map(|a| a.to_string()))
    }

    pub fn rebroadcast_all_events(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.rebroadcast_all_events().await?) })
    }

    pub fn republish_shared_key_for_policy(&self, policy_id: String) -> Result<()> {
        block_on(async move {
            let policy_id = EventId::from_hex(policy_id)?;
            Ok(self
                .inner
                .republish_shared_key_for_policy(policy_id)
                .await?)
        })
    }

    pub fn approve_nostr_connect_request(&self, event_id: String) -> Result<()> {
        let event_id = EventId::from_hex(event_id)?;
        block_on(async move { Ok(self.inner.approve_nostr_connect_request(event_id).await?) })
    }
}
