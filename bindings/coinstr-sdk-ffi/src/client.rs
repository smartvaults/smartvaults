// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::HashMap;
use std::fmt::Debug;
use std::ops::Deref;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_utility::thread;
use coinstr_sdk::client;
use coinstr_sdk::core::bips::bip39::Mnemonic;
use coinstr_sdk::core::bitcoin::psbt::PartiallySignedTransaction;
use coinstr_sdk::core::bitcoin::{Address, Network, Txid, XOnlyPublicKey};
use coinstr_sdk::core::types::{FeeRate, Priority, WordCount};
use coinstr_sdk::db::model::{GetApprovedProposalResult, GetProposal as GetProposalSdk};
use coinstr_sdk::nostr::{self, block_on};
use nostr_ffi::{EventId, Keys, PublicKey};

use crate::error::Result;
use crate::{
    AbortHandle, AddressIndex, Amount, Approval, Balance, CompletedProposal, Config, GetAddress,
    GetCompletedProposal, GetPolicy, GetProposal, GetTransaction, KeychainSeed, Message, Metadata,
    NostrConnectRequest, NostrConnectSession, NostrConnectURI, OutPoint, Relay, Signer, Utxo,
};

pub struct Coinstr {
    inner: client::Coinstr,
    dropped: AtomicBool,
}

impl Drop for Coinstr {
    fn drop(&mut self) {
        if self.dropped.load(Ordering::SeqCst) {
            tracing::warn!("Coinstr already dropped");
        } else {
            tracing::debug!("Dropping Coinstr client...");
            let _ = self
                .dropped
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |_| Some(true));
            let inner = self.inner.clone();
            thread::spawn(async move {
                inner
                    .shutdown()
                    .await
                    .expect("Impossible to drop Coinstr client")
            });
        }
    }
}

impl Coinstr {
    /// Open keychain
    pub fn open(
        base_path: String,
        name: String,
        password: String,
        network: Network,
    ) -> Result<Self> {
        block_on(async move {
            Ok(Self {
                inner: client::Coinstr::open(base_path, name, || Ok(password), network).await?,
                dropped: AtomicBool::new(false),
            })
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
        block_on(async move {
            Ok(Self {
                inner: client::Coinstr::generate(
                    base_path,
                    name,
                    || Ok(password),
                    word_count,
                    || Ok(passphrase),
                    network,
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
        mnemonic: String,
        passphrase: Option<String>,
        network: Network,
    ) -> Result<Self> {
        block_on(async move {
            let mnemonic = Mnemonic::from_str(&mnemonic)?;
            Ok(Self {
                inner: client::Coinstr::restore(
                    base_path,
                    name,
                    || Ok(password),
                    || Ok(mnemonic),
                    || Ok(passphrase),
                    network,
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

    /// Save keychain
    pub fn save(&self) -> Result<()> {
        Ok(self.inner.save()?)
    }

    /// Check keychain password
    pub fn check_password(&self, password: String) -> bool {
        self.inner.check_password(password)
    }

    pub fn rename(&self, new_name: String) -> Result<()> {
        Ok(self.inner.rename(new_name)?)
    }

    /// Change keychain password
    pub fn change_password(&self, new_password: String) -> Result<()> {
        Ok(self.inner.change_password(|| Ok(new_password))?)
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

    pub fn seed(&self) -> Arc<KeychainSeed> {
        Arc::new(self.inner.keychain().seed().into())
    }

    pub fn keys(&self) -> Arc<Keys> {
        Arc::new(self.inner.keys().into())
    }

    pub fn network(&self) -> Network {
        self.inner.network()
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

    pub fn set_metadata(&self, json: String) -> Result<()> {
        block_on(async move {
            let metadata = nostr::Metadata::from_json(json)?;
            Ok(self.inner.set_metadata(metadata).await?)
        })
    }

    pub fn get_profile(&self) -> Result<Arc<Metadata>> {
        Ok(Arc::new(self.inner.get_profile()?.into()))
    }

    // TODO: return PublicKey instead of String (must replace HashMap with Vec)
    pub fn get_contacts(&self) -> Result<HashMap<String, Arc<Metadata>>> {
        Ok(self
            .inner
            .get_contacts()?
            .into_iter()
            .map(|(pk, m)| (pk.to_string(), Arc::new(m.into())))
            .collect())
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
        Ok(Arc::new(self.inner.get_policy_by_id(**policy_id)?.into()))
    }

    pub fn get_proposal_by_id(&self, proposal_id: Arc<EventId>) -> Result<Arc<GetProposal>> {
        Ok(Arc::new(
            self.inner.get_proposal_by_id(**proposal_id)?.into(),
        ))
    }

    pub fn get_completed_proposal_by_id(
        &self,
        completed_proposal_id: Arc<EventId>,
    ) -> Result<Arc<GetCompletedProposal>> {
        Ok(Arc::new(
            self.inner
                .get_completed_proposal_by_id(**completed_proposal_id)?
                .into(),
        ))
    }

    pub fn get_signer_by_id(&self, signer_id: Arc<EventId>) -> Result<Arc<Signer>> {
        Ok(Arc::new(self.inner.get_signer_by_id(**signer_id)?.into()))
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
        let policies = self.inner.get_policies()?;
        Ok(policies.into_iter().map(|p| Arc::new(p.into())).collect())
    }

    pub fn get_proposals(&self) -> Result<Vec<Arc<GetProposal>>> {
        let proposals = self.inner.get_proposals()?;
        Ok(proposals.into_iter().map(|p| Arc::new(p.into())).collect())
    }

    pub fn get_proposals_by_policy_id(
        &self,
        policy_id: Arc<EventId>,
    ) -> Result<Vec<Arc<GetProposal>>> {
        let proposals = self.inner.get_proposals_by_policy_id(**policy_id)?;
        Ok(proposals.into_iter().map(|p| Arc::new(p.into())).collect())
    }

    pub fn is_proposal_signed(&self, proposal_id: Arc<EventId>) -> Result<bool> {
        let GetProposalSdk { proposal, .. } = self.inner.get_proposal_by_id(**proposal_id)?;
        let approvals = self
            .inner
            .get_approvals_by_proposal_id(**proposal_id)?
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

    // TODO: replace String with EventId (replace HashMap with Vec)
    pub fn get_approvals_by_proposal_id(
        &self,
        proposal_id: Arc<EventId>,
    ) -> Result<HashMap<String, Arc<Approval>>> {
        Ok(self
            .inner
            .get_approvals_by_proposal_id(**proposal_id)?
            .into_iter()
            .map(|(id, res)| (id.to_hex(), Arc::new(res.into())))
            .collect())
    }

    pub fn get_completed_proposals(&self) -> Result<Vec<Arc<GetCompletedProposal>>> {
        let completed_proposals = self.inner.get_completed_proposals()?;
        Ok(completed_proposals
            .into_iter()
            .map(|p| Arc::new(p.into()))
            .collect())
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

    pub fn spend(
        &self,
        policy_id: Arc<EventId>,
        to_address: String,
        amount: Arc<Amount>,
        description: String,
        target_blocks: u8,
        utxos: Option<Vec<Arc<OutPoint>>>,
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
                    None,
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
                    None,
                )
                .await?;
            Ok(Arc::new(proposal.into()))
        })
    }

    pub fn approve(&self, proposal_id: Arc<EventId>) -> Result<Arc<EventId>> {
        block_on(async move {
            let (approval_id, ..) = self.inner.approve(**proposal_id).await?;
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

    pub fn coinstr_signer_exists(&self) -> Result<bool> {
        Ok(self.inner.coinstr_signer_exists()?)
    }

    pub fn save_coinstr_signer(&self) -> Result<Arc<EventId>> {
        block_on(async move { Ok(Arc::new(self.inner.save_coinstr_signer().await?.into())) })
    }

    // TODO: add get_all_signers

    // TODO: replace String with EventId
    pub fn get_signers(&self) -> Result<HashMap<String, Arc<Signer>>> {
        Ok(self
            .inner
            .get_signers()?
            .into_iter()
            .map(|(id, s)| (id.to_hex(), Arc::new(s.into())))
            .collect())
    }

    pub fn get_balance(&self, policy_id: Arc<EventId>) -> Option<Arc<Balance>> {
        self.inner
            .get_balance(**policy_id)
            .map(|b| Arc::new(b.into()))
    }

    pub fn get_txs(&self, policy_id: Arc<EventId>) -> Result<Vec<Arc<GetTransaction>>> {
        Ok(self
            .inner
            .get_txs(**policy_id)?
            .into_iter()
            .map(|tx| Arc::new(tx.into()))
            .collect())
    }

    pub fn get_tx(&self, policy_id: Arc<EventId>, txid: String) -> Result<Arc<GetTransaction>> {
        let txid = Txid::from_str(&txid)?;
        Ok(self
            .inner
            .get_tx(**policy_id, txid)
            .map(|tx| Arc::new(tx.into()))?)
    }

    pub fn get_utxos(&self, policy_id: Arc<EventId>) -> Result<Vec<Arc<Utxo>>> {
        Ok(self
            .inner
            .get_utxos(**policy_id)?
            .into_iter()
            .map(|u| Arc::new(u.into()))
            .collect())
    }

    pub fn get_total_balance(&self) -> Result<Arc<Balance>> {
        Ok(Arc::new(self.inner.get_total_balance()?.into()))
    }

    pub fn get_all_txs(&self) -> Result<Vec<Arc<GetTransaction>>> {
        Ok(self
            .inner
            .get_all_transactions()?
            .into_iter()
            .map(|tx| Arc::new(tx.into()))
            .collect())
    }

    pub fn get_address(
        &self,
        policy_id: Arc<EventId>,
        index: AddressIndex,
    ) -> Result<Arc<GetAddress>> {
        let address = self.inner.get_address(**policy_id, index.into())?;
        Ok(Arc::new(address.into()))
    }

    pub fn get_last_unused_address(&self, policy_id: Arc<EventId>) -> Result<Arc<GetAddress>> {
        let address = self.inner.get_last_unused_address(**policy_id)?;
        Ok(Arc::new(address.into()))
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
        Ok(self
            .inner
            .get_nostr_connect_sessions()?
            .into_iter()
            .map(|(uri, timestamp)| NostrConnectSession {
                uri: Arc::new(uri.into()),
                timestamp: timestamp.as_u64(),
            })
            .collect())
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
        Ok(self
            .inner
            .get_nostr_connect_requests(approved)?
            .into_iter()
            .map(|req| Arc::new(req.into()))
            .collect())
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
    ) -> Result<()> {
        self.inner
            .auto_approve_nostr_connect_requests(**app_public_key, duration);
        Ok(())
    }

    // TODO: add revoke_nostr_connect_auto_approve

    // TODO: add get_nostr_connect_pre_authorizations

    pub fn handle_sync(self: Arc<Self>, handler: Box<dyn SyncHandler>) -> Arc<AbortHandle> {
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
