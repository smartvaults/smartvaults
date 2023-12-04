// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_utility::thread;
use nostr_sdk_ffi::{EventId, Keys, Metadata, PublicKey};
use smartvaults_sdk::client;
use smartvaults_sdk::core::bips::bip39::Mnemonic;
use smartvaults_sdk::core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_sdk::core::bitcoin::{Address, Txid};
use smartvaults_sdk::core::miniscript::Descriptor;
use smartvaults_sdk::core::secp256k1::XOnlyPublicKey;
use smartvaults_sdk::core::types::{FeeRate, Priority, WordCount};
use smartvaults_sdk::nostr::block_on;

use crate::error::Result;
use crate::{
    AbortHandle, AddressIndex, Amount, Balance, CompletedProposal, Config, GetAddress, GetApproval,
    GetCompletedProposal, GetPolicy, GetProposal, GetSharedSigner, GetSigner, GetTransaction,
    KeyAgent, KeychainSeed, Message, Network, NostrConnectRequest, NostrConnectSession,
    NostrConnectURI, OutPoint, Period, PolicyTemplate, Relay, Signer, SignerOffering, User, Utxo,
};

pub struct SmartVaults {
    inner: client::SmartVaults,
    dropped: AtomicBool,
}

impl Drop for SmartVaults {
    fn drop(&mut self) {
        if self.dropped.load(Ordering::SeqCst) {
            tracing::warn!("SmartVaults client already dropped");
        } else {
            tracing::info!("Dropping SmartVaults client...");
            let _ = self
                .dropped
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(true));
            let inner = self.inner.clone();
            thread::spawn(async move {
                inner
                    .shutdown()
                    .await
                    .expect("Impossible to drop SmartVaults client")
            });
        }
    }
}

impl SmartVaults {
    /// Open keychain
    pub fn open(
        base_path: String,
        name: String,
        password: String,
        network: Network,
    ) -> Result<Self> {
        block_on(async move {
            Ok(Self {
                inner: client::SmartVaults::open(base_path, name, password, network.into()).await?,
                dropped: AtomicBool::new(false),
            })
        })
    }

    /// Generate keychain
    pub fn generate(
        base_path: String,
        name: String,
        password: String,
        confirm_password: String,
        word_count: WordCount,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        block_on(async move {
            Ok(Self {
                inner: client::SmartVaults::generate(
                    base_path,
                    name,
                    || Ok(password),
                    || Ok(confirm_password),
                    word_count,
                    || Ok(passphrase),
                    network.into(),
                )
                .await?,
                dropped: AtomicBool::new(false),
            })
        })
    }

    /// Restore keychain
    pub fn restore(
        base_path: String,
        name: String,
        password: String,
        confirm_password: String,
        mnemonic: String,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        block_on(async move {
            let mnemonic = Mnemonic::from_str(&mnemonic)?;
            Ok(Self {
                inner: client::SmartVaults::restore(
                    base_path,
                    name,
                    || Ok(password),
                    || Ok(confirm_password),
                    || Ok(mnemonic),
                    || Ok(passphrase),
                    network.into(),
                )
                .await?,
                dropped: AtomicBool::new(false),
            })
        })
    }

    /// Get keychain name
    pub fn name(&self) -> Option<String> {
        self.inner.name()
    }

    /// Check keychain password
    pub fn check_password(&self, password: String) -> bool {
        self.inner.check_password(password)
    }

    pub fn rename(&self, new_name: String) -> Result<()> {
        Ok(self.inner.rename(new_name)?)
    }

    /// Change keychain password
    pub fn change_password(
        &self,
        password: String,
        new_password: String,
        confirm_password: String,
    ) -> Result<()> {
        Ok(self.inner.change_password(
            || Ok(password),
            || Ok(new_password),
            || Ok(confirm_password),
        )?)
    }

    /// Permanent delete the keychain
    pub fn wipe(&self, password: String) -> Result<()> {
        Ok(self.inner.wipe(password)?)
    }

    pub fn start(&self) {
        block_on(async move { self.inner.start().await })
    }

    pub fn stop(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.stop().await?) })
    }

    pub fn clear_cache(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.clear_cache().await?) })
    }

    pub fn seed(&self, password: String) -> Result<Arc<KeychainSeed>> {
        Ok(Arc::new(self.inner.keychain(password)?.seed().into()))
    }

    pub fn keys(&self) -> Arc<Keys> {
        block_on(async move { Arc::new(self.inner.keys().await.into()) })
    }

    pub fn network(&self) -> Network {
        self.inner.network().into()
    }

    /// Add new relay
    pub fn add_relay(&self, url: String) -> Result<()> {
        block_on(async move { Ok(self.inner.add_relay(url, None).await?) })
    }

    pub fn default_relays(&self) -> Vec<String> {
        self.inner.default_relays()
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

    pub fn config(&self) -> Arc<Config> {
        Arc::new(self.inner.config().into())
    }

    pub fn block_height(&self) -> u32 {
        self.inner.block_height()
    }

    pub fn set_metadata(&self, metadata: Arc<Metadata>) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .set_metadata(metadata.as_ref().deref().clone())
                .await?)
        })
    }

    pub fn get_profile(&self) -> Result<Arc<User>> {
        block_on(async move { Ok(Arc::new(self.inner.get_profile().await?.into())) })
    }

    pub fn get_public_key_metadata(&self, public_key: Arc<PublicKey>) -> Result<Arc<Metadata>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .get_public_key_metadata(**public_key)
                    .await?
                    .into(),
            ))
        })
    }

    pub fn get_contacts(&self) -> Result<Vec<Arc<User>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_contacts()
                .await?
                .into_iter()
                .map(|user| Arc::new(user.into()))
                .collect())
        })
    }

    /// Add new contact
    pub fn add_contact(&self, public_key: Arc<PublicKey>) -> Result<()> {
        block_on(async move { Ok(self.inner.add_contact(**public_key).await?) })
    }

    /// Remove contact
    pub fn remove_contact(&self, public_key: Arc<PublicKey>) -> Result<()> {
        block_on(async move { Ok(self.inner.remove_contact(**public_key).await?) })
    }

    pub fn get_policy_by_id(&self, policy_id: Arc<EventId>) -> Result<Arc<GetPolicy>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner.get_policy_by_id(**policy_id).await?.into(),
            ))
        })
    }

    pub fn get_proposal_by_id(&self, proposal_id: Arc<EventId>) -> Result<Arc<GetProposal>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner.get_proposal_by_id(**proposal_id).await?.into(),
            ))
        })
    }

    pub fn get_completed_proposal_by_id(
        &self,
        completed_proposal_id: Arc<EventId>,
    ) -> Result<Arc<GetCompletedProposal>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .get_completed_proposal_by_id(**completed_proposal_id)
                    .await?
                    .into(),
            ))
        })
    }

    pub fn get_signer_by_id(&self, signer_id: Arc<EventId>) -> Result<Arc<Signer>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner.get_signer_by_id(**signer_id).await?.into(),
            ))
        })
    }

    pub fn delete_policy_by_id(&self, policy_id: Arc<EventId>) -> Result<()> {
        block_on(async move { Ok(self.inner.delete_policy_by_id(**policy_id).await?) })
    }

    pub fn delete_proposal_by_id(&self, proposal_id: Arc<EventId>) -> Result<()> {
        block_on(async move { Ok(self.inner.delete_proposal_by_id(**proposal_id).await?) })
    }

    pub fn delete_completed_proposal_by_id(
        &self,
        completed_proposal_id: Arc<EventId>,
    ) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .delete_completed_proposal_by_id(**completed_proposal_id)
                .await?)
        })
    }

    pub fn delete_signer_by_id(&self, signer_id: Arc<EventId>) -> Result<()> {
        block_on(async move { Ok(self.inner.delete_signer_by_id(**signer_id).await?) })
    }

    pub fn get_policies(&self) -> Result<Vec<Arc<GetPolicy>>> {
        block_on(async move {
            let policies = self.inner.get_policies().await?;
            Ok(policies.into_iter().map(|p| Arc::new(p.into())).collect())
        })
    }

    pub fn get_proposals(&self) -> Result<Vec<Arc<GetProposal>>> {
        block_on(async move {
            let proposals = self.inner.get_proposals().await?;
            Ok(proposals.into_iter().map(|p| Arc::new(p.into())).collect())
        })
    }

    pub fn get_proposals_by_policy_id(
        &self,
        policy_id: Arc<EventId>,
    ) -> Result<Vec<Arc<GetProposal>>> {
        block_on(async move {
            let proposals = self.inner.get_proposals_by_policy_id(**policy_id).await?;
            Ok(proposals.into_iter().map(|p| Arc::new(p.into())).collect())
        })
    }

    pub fn get_approvals_by_proposal_id(
        &self,
        proposal_id: Arc<EventId>,
    ) -> Result<Vec<Arc<GetApproval>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_approvals_by_proposal_id(**proposal_id)
                .await?
                .into_iter()
                .map(|res| Arc::new(res.into()))
                .collect())
        })
    }

    pub fn get_completed_proposals(&self) -> Result<Vec<Arc<GetCompletedProposal>>> {
        block_on(async move {
            let completed_proposals = self.inner.get_completed_proposals().await?;
            Ok(completed_proposals
                .into_iter()
                .map(|p| Arc::new(p.into()))
                .collect())
        })
    }

    pub fn get_members_of_policy(&self, policy_id: Arc<EventId>) -> Result<Vec<Arc<User>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_members_of_policy(**policy_id)
                .await?
                .into_iter()
                .map(|u| Arc::new(u.into()))
                .collect())
        })
    }

    pub fn save_policy(
        &self,
        name: String,
        description: String,
        descriptor: String,
        public_keys: Vec<Arc<PublicKey>>,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            let nostr_pubkeys: Vec<XOnlyPublicKey> = public_keys.into_iter().map(|p| **p).collect();
            Ok(Arc::new(
                self.inner
                    .save_policy(name, description, descriptor, nostr_pubkeys)
                    .await?
                    .into(),
            ))
        })
    }

    pub fn save_policy_from_template(
        &self,
        name: String,
        description: String,
        template: Arc<PolicyTemplate>,
        public_keys: Vec<Arc<PublicKey>>,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            let nostr_pubkeys: Vec<XOnlyPublicKey> = public_keys.into_iter().map(|p| **p).collect();
            Ok(Arc::new(
                self.inner
                    .save_policy_from_template(
                        name,
                        description,
                        template.as_ref().deref().clone(),
                        nostr_pubkeys,
                    )
                    .await?
                    .into(),
            ))
        })
    }

    pub fn spend(
        &self,
        policy_id: Arc<EventId>,
        to_address: String,
        amount: Arc<Amount>,
        description: String,
        target_blocks: u8,
        utxos: Option<Vec<Arc<OutPoint>>>,
        policy_path: Option<HashMap<String, Vec<u64>>>,
        skip_frozen_utxos: bool,
    ) -> Result<Arc<GetProposal>> {
        block_on(async move {
            let to_address = Address::from_str(&to_address)?;
            let proposal = self
                .inner
                .spend(
                    **policy_id,
                    to_address,
                    amount.inner(),
                    description,
                    FeeRate::Priority(Priority::Custom(target_blocks)),
                    utxos.map(|utxos| utxos.into_iter().map(|u| u.as_ref().into()).collect()),
                    policy_path.map(|pp| {
                        pp.into_iter()
                            .map(|(k, v)| (k, v.into_iter().map(|i| i as usize).collect()))
                            .collect()
                    }),
                    skip_frozen_utxos,
                )
                .await?;
            Ok(Arc::new(proposal.into()))
        })
    }

    pub fn self_transfer(
        &self,
        from_policy_id: Arc<EventId>,
        to_policy_id: Arc<EventId>,
        amount: Arc<Amount>,
        target_blocks: u8,
        utxos: Option<Vec<Arc<OutPoint>>>,
        policy_path: Option<HashMap<String, Vec<u64>>>,
        skip_frozen_utxos: bool,
    ) -> Result<Arc<GetProposal>> {
        block_on(async move {
            let proposal = self
                .inner
                .self_transfer(
                    **from_policy_id,
                    **to_policy_id,
                    amount.inner(),
                    FeeRate::Priority(Priority::Custom(target_blocks)),
                    utxos.map(|utxos| utxos.into_iter().map(|u| u.as_ref().into()).collect()),
                    policy_path.map(|pp| {
                        pp.into_iter()
                            .map(|(k, v)| (k, v.into_iter().map(|i| i as usize).collect()))
                            .collect()
                    }),
                    skip_frozen_utxos,
                )
                .await?;
            Ok(Arc::new(proposal.into()))
        })
    }

    pub fn approve(&self, password: String, proposal_id: Arc<EventId>) -> Result<Arc<EventId>> {
        block_on(async move {
            let (approval_id, ..) = self.inner.approve(password, **proposal_id).await?;
            Ok(Arc::new(approval_id.into()))
        })
    }

    pub fn approve_with_signed_psbt(
        &self,
        proposal_id: Arc<EventId>,
        signed_psbt: String,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            let signed_psbt = PartiallySignedTransaction::from_str(&signed_psbt)?;
            let (approval_id, ..) = self
                .inner
                .approve_with_signed_psbt(**proposal_id, signed_psbt)
                .await?;
            Ok(Arc::new(approval_id.into()))
        })
    }

    pub fn revoke_approval(&self, approval_id: Arc<EventId>) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .revoke_approval(approval_id.as_ref().into())
                .await?)
        })
    }

    pub fn finalize(&self, proposal_id: Arc<EventId>) -> Result<CompletedProposal> {
        block_on(async move { Ok(self.inner.finalize(**proposal_id).await?.into()) })
    }

    pub fn new_proof_proposal(
        &self,
        policy_id: Arc<EventId>,
        message: String,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .new_proof_proposal(**policy_id, message)
                    .await?
                    .0
                    .into(),
            ))
        })
    }

    // TODO: add verify_proof

    // TODO: add verify_proof_by_id

    // TODO: add save_signer

    pub fn smartvaults_signer_exists(&self) -> Result<bool> {
        block_on(async move { Ok(self.inner.smartvaults_signer_exists().await?) })
    }

    pub fn save_smartvaults_signer(&self) -> Result<Arc<EventId>> {
        block_on(async move { Ok(Arc::new(self.inner.save_smartvaults_signer().await?.into())) })
    }

    // TODO: add get_all_signers

    pub fn get_signers(&self) -> Result<Vec<Arc<GetSigner>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_signers()
                .await?
                .into_iter()
                .map(|s| Arc::new(s.into()))
                .collect())
        })
    }

    pub fn share_signer(
        &self,
        signer_id: Arc<EventId>,
        public_key: Arc<PublicKey>,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .share_signer(**signer_id, **public_key)
                    .await?
                    .into(),
            ))
        })
    }

    pub fn share_signer_to_multiple_public_keys(
        &self,
        signer_id: Arc<EventId>,
        public_keys: Vec<Arc<PublicKey>>,
    ) -> Result<()> {
        block_on(async move {
            let public_keys: Vec<XOnlyPublicKey> = public_keys.into_iter().map(|p| **p).collect();
            Ok(self
                .inner
                .share_signer_to_multiple_public_keys(**signer_id, public_keys)
                .await?)
        })
    }

    pub fn revoke_all_shared_signers(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.revoke_all_shared_signers().await?) })
    }

    pub fn revoke_shared_signer(&self, shared_signer_id: Arc<EventId>) -> Result<()> {
        block_on(async move { Ok(self.inner.revoke_shared_signer(**shared_signer_id).await?) })
    }

    pub fn get_shared_signers(&self) -> Result<Vec<Arc<GetSharedSigner>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_shared_signers()
                .await?
                .into_iter()
                .map(|s| Arc::new(s.into()))
                .collect())
        })
    }

    pub fn get_shared_signers_public_keys(
        &self,
        include_contacts: bool,
    ) -> Result<Vec<Arc<PublicKey>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_shared_signers_public_keys(include_contacts)
                .await?
                .into_iter()
                .map(|p| Arc::new(p.into()))
                .collect())
        })
    }

    pub fn get_shared_signers_by_public_key(
        &self,
        public_key: Arc<PublicKey>,
    ) -> Result<Vec<Arc<GetSharedSigner>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_shared_signers_by_public_key(**public_key)
                .await?
                .into_iter()
                .map(|s| Arc::new(s.into()))
                .collect())
        })
    }

    pub fn get_balance(&self, policy_id: Arc<EventId>) -> Option<Arc<Balance>> {
        block_on(async move {
            #[allow(deprecated)]
            self.inner
                .get_balance(**policy_id)
                .await
                .map(|b| Arc::new(b.into()))
        })
    }

    pub fn get_txs(&self, policy_id: Arc<EventId>) -> Result<Vec<Arc<GetTransaction>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_txs(**policy_id, true)
                .await?
                .into_iter()
                .map(|tx| Arc::new(tx.into()))
                .collect())
        })
    }

    pub fn get_tx(&self, policy_id: Arc<EventId>, txid: String) -> Result<Arc<GetTransaction>> {
        block_on(async move {
            let txid = Txid::from_str(&txid)?;
            Ok(self
                .inner
                .get_tx(**policy_id, txid)
                .await
                .map(|tx| Arc::new(tx.into()))?)
        })
    }

    pub fn get_utxos(&self, policy_id: Arc<EventId>) -> Result<Vec<Arc<Utxo>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_utxos(**policy_id)
                .await?
                .into_iter()
                .map(|u| Arc::new(u.into()))
                .collect())
        })
    }

    pub fn get_total_balance(&self) -> Result<Arc<Balance>> {
        block_on(async move { Ok(Arc::new(self.inner.get_total_balance().await?.into())) })
    }

    pub fn get_all_txs(&self) -> Result<Vec<Arc<GetTransaction>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_all_transactions()
                .await?
                .into_iter()
                .map(|tx| Arc::new(tx.into()))
                .collect())
        })
    }

    pub fn get_address(
        &self,
        policy_id: Arc<EventId>,
        index: AddressIndex,
    ) -> Result<Arc<GetAddress>> {
        block_on(async move {
            let address = self.inner.get_address(**policy_id, index.into()).await?;
            Ok(Arc::new(address.into()))
        })
    }

    pub fn get_last_unused_address(&self, policy_id: Arc<EventId>) -> Result<Arc<GetAddress>> {
        block_on(async move {
            let address = self.inner.get_last_unused_address(**policy_id).await?;
            Ok(Arc::new(address.into()))
        })
    }

    pub fn rebroadcast_all_events(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.rebroadcast_all_events().await?) })
    }

    pub fn republish_shared_key_for_policy(&self, policy_id: Arc<EventId>) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .republish_shared_key_for_policy(**policy_id)
                .await?)
        })
    }

    // TODO: add notifications methods

    pub fn new_nostr_connect_session(&self, uri: Arc<NostrConnectURI>) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .new_nostr_connect_session(uri.as_ref().deref().clone())
                .await?)
        })
    }

    pub fn get_nostr_connect_sessions(&self) -> Result<Vec<NostrConnectSession>> {
        block_on(async move {
            Ok(self
                .inner
                .get_nostr_connect_sessions()
                .await?
                .into_iter()
                .map(|(uri, timestamp)| NostrConnectSession {
                    uri: Arc::new(uri.into()),
                    timestamp: timestamp.as_u64(),
                })
                .collect())
        })
    }

    pub fn disconnect_nostr_connect_session(&self, app_public_key: Arc<PublicKey>) -> Result<()> {
        block_on(async move {
            Ok(self
                .inner
                .disconnect_nostr_connect_session(**app_public_key)
                .await?)
        })
    }

    pub fn get_nostr_connect_requests(
        &self,
        approved: bool,
    ) -> Result<Vec<Arc<NostrConnectRequest>>> {
        block_on(async move {
            Ok(self
                .inner
                .get_nostr_connect_requests(approved)
                .await?
                .into_iter()
                .map(|req| Arc::new(req.into()))
                .collect())
        })
    }

    pub fn approve_nostr_connect_request(&self, event_id: Arc<EventId>) -> Result<()> {
        block_on(async move { Ok(self.inner.approve_nostr_connect_request(**event_id).await?) })
    }

    pub fn reject_nostr_connect_request(&self, event_id: Arc<EventId>) -> Result<()> {
        block_on(async move { Ok(self.inner.reject_nostr_connect_request(**event_id).await?) })
    }

    pub fn auto_approve_nostr_connect_requests(
        &self,
        app_public_key: Arc<PublicKey>,
        duration: Duration,
    ) {
        block_on(async move {
            self.inner
                .auto_approve_nostr_connect_requests(**app_public_key, duration)
                .await;
        })
    }

    // TODO: add revoke_nostr_connect_auto_approve

    // TODO: add get_nostr_connect_pre_authorizations

    pub fn announce_key_agent(&self) -> Result<Arc<EventId>> {
        block_on(async move { Ok(Arc::new(self.inner.announce_key_agent().await?.into())) })
    }

    pub fn deannounce_key_agent(&self) -> Result<()> {
        block_on(async move { Ok(self.inner.deannounce_key_agent().await?) })
    }

    pub fn signer_offering(
        &self,
        signer: Arc<Signer>,
        offering: SignerOffering,
    ) -> Result<Arc<EventId>> {
        block_on(async move {
            Ok(Arc::new(
                self.inner
                    .signer_offering(&signer, offering.into())
                    .await?
                    .into(),
            ))
        })
    }

    pub fn key_agents(&self) -> Result<Vec<KeyAgent>> {
        block_on(async move {
            Ok(self
                .inner
                .key_agents()
                .await?
                .into_iter()
                .map(|k| k.into())
                .collect())
        })
    }

    pub fn request_signers_to_key_agent(&self, key_agent: Arc<PublicKey>) -> Result<()> {
        self.add_contact(key_agent)
    }

    pub fn key_agent_payment(
        &self,
        policy_id: Arc<EventId>,
        to_address: String,
        amount: Arc<Amount>,
        description: String,
        signer_descriptor: String,
        period: Period,
        target_blocks: u8,
        utxos: Option<Vec<Arc<OutPoint>>>,
        policy_path: Option<HashMap<String, Vec<u64>>>,
        skip_frozen_utxos: bool,
    ) -> Result<Arc<GetProposal>> {
        block_on(async move {
            let to_address = Address::from_str(&to_address)?;
            let proposal = self
                .inner
                .key_agent_payment(
                    **policy_id,
                    to_address,
                    amount.inner(),
                    description,
                    Descriptor::from_str(&signer_descriptor)?,
                    period.into(),
                    FeeRate::Priority(Priority::Custom(target_blocks)),
                    utxos.map(|utxos| utxos.into_iter().map(|u| u.as_ref().into()).collect()),
                    policy_path.map(|pp| {
                        pp.into_iter()
                            .map(|(k, v)| (k, v.into_iter().map(|i| i as usize).collect()))
                            .collect()
                    }),
                    skip_frozen_utxos,
                )
                .await?;
            Ok(Arc::new(proposal.into()))
        })
    }

    pub fn handle_sync(self: Arc<Self>, handler: Box<dyn SyncHandler>) -> Arc<AbortHandle> {
        tracing::info!("Spawning new `handle_sync` thread");
        let handle = async_utility::thread::abortable(async move {
            let mut receiver = self.inner.sync_notifications();
            let handler = Arc::new(handler);
            while let Ok(message) = receiver.recv().await {
                let h = handler.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    h.handle(message.into());
                })
                .await;
            }
        });

        Arc::new(handle.into())
    }
}

pub trait SyncHandler: Send + Sync + Debug {
    fn handle(&self, msg: Message);
}
