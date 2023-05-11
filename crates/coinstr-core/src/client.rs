// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap};
use std::net::SocketAddr;
use std::ops::Add;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use bdk::bitcoin::psbt::PartiallySignedTransaction;
use bdk::bitcoin::{Address, Network, PrivateKey, Txid, XOnlyPublicKey};
use bdk::blockchain::Blockchain;
use bdk::blockchain::ElectrumBlockchain;
use bdk::database::MemoryDatabase;
use bdk::electrum_client::Client as ElectrumClient;
use bdk::miniscript::psbt::PsbtExt;
use bdk::signer::{SignerContext, SignerOrdering, SignerWrapper};
use bdk::{KeychainKind, SignOptions, SyncOptions, TransactionDetails, Wallet};
use futures_util::future::{AbortHandle, Abortable};
use keechain_core::bip39::Mnemonic;
use keechain_core::types::{KeeChain, Keychain, WordCount};
use nostr_sdk::nips::nip06::FromMnemonic;
use nostr_sdk::prelude::TagKind;
use nostr_sdk::secp256k1::SecretKey;
use nostr_sdk::{
    nips, Client, Event, EventBuilder, EventId, Filter, Keys, Kind, Metadata, Relay,
    RelayPoolNotification, Result, Tag, Timestamp, Url, SECP256K1,
};
use tokio::sync::mpsc::{self, Receiver, Sender};
use tokio::sync::RwLock;

use crate::cache::{Cache, GetApprovedProposals};
use crate::constants::{
    APPROVED_PROPOSAL_EXPIRATION, APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, POLICY_KIND,
    PROPOSAL_KIND, SHARED_KEY_KIND,
};
use crate::policy::Policy;
use crate::proposal::{ApprovedProposal, CompletedProposal, Proposal};
use crate::reserves::ProofOfReserves;
use crate::util::encryption::{Encryption, EncryptionError};
use crate::{thread, util, Amount, FeeRate};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Keechain(#[from] keechain_core::types::keychain::Error),
    #[error(transparent)]
    Bdk(#[from] bdk::Error),
    #[error(transparent)]
    Electrum(#[from] bdk::electrum_client::Error),
    #[error(transparent)]
    Client(#[from] nostr_sdk::client::Error),
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr_sdk::event::builder::Error),
    #[error(transparent)]
    NIP04(#[from] nostr_sdk::nips::nip04::Error),
    #[error(transparent)]
    Policy(#[from] crate::policy::Error),
    #[error(transparent)]
    Secp256k1(#[from] keechain_core::bitcoin::secp256k1::Error),
    #[error(transparent)]
    Encryption(#[from] EncryptionError),
    #[error(transparent)]
    Psbt(#[from] keechain_core::bitcoin::psbt::Error),
    #[error(transparent)]
    PsbtParse(#[from] keechain_core::bitcoin::psbt::PsbtParseError),
    #[error(transparent)]
    Util(#[from] crate::util::Error),
    #[error(transparent)]
    NIP06(#[from] nostr_sdk::nips::nip06::Error),
    #[error(transparent)]
    Cache(#[from] crate::cache::Error),
    #[error(transparent)]
    ProofOfReserves(#[from] crate::reserves::ProofError),
    #[error("shared keys not found")]
    SharedKeysNotFound,
    #[error("policy not found")]
    PolicyNotFound,
    #[error("proposal not found")]
    ProposalNotFound,
    #[error("unexpected proposal")]
    UnexpectedProposal,
    #[error("approved proposal/s not found")]
    ApprovedProposalNotFound,
    #[error("impossible to finalize the PSBT: {0:?}")]
    ImpossibleToFinalizePsbt(Vec<bdk::miniscript::psbt::Error>),
    #[error("impossible to finalize the non-std PSBT")]
    ImpossibleToFinalizeNonStdPsbt,
    #[error("PSBT not signed")]
    PsbtNotSigned,
    #[error("wallet spending policy not found")]
    WalletSpendingPolicyNotFound,
    #[error("electrum endpoint not set")]
    ElectrumEndpointNotSet,
    #[error("{0}")]
    Generic(String),
}

/// Coinstr
#[derive(Debug, Clone)]
pub struct Coinstr {
    network: Network,
    keechain: KeeChain,
    client: Client,
    endpoint: Arc<RwLock<Option<String>>>,
    pub cache: Cache,
}

impl Coinstr {
    /// Open keychain
    pub fn open<P, PSW>(path: P, get_password: PSW, network: Network) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        PSW: FnOnce() -> Result<String>,
    {
        // Open keychain
        let mut keechain: KeeChain = KeeChain::open(path, get_password)?;
        let passphrase: Option<String> = keechain.keychain.get_passphrase(0);
        keechain.keychain.apply_passphrase(passphrase);

        // Get nostr keys
        let keys = Keys::from_mnemonic(
            keechain.keychain.seed.mnemonic().to_string(),
            keechain.keychain.seed.passphrase(),
        )?;

        Ok(Self {
            network,
            keechain,
            client: Client::new(&keys),
            endpoint: Arc::new(RwLock::new(None)),
            cache: Cache::new(),
        })
    }

    /// Generate keychain
    pub fn generate<P, PSW, PASSP>(
        path: P,
        get_password: PSW,
        word_count: WordCount,
        get_passphrase: PASSP,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        PSW: FnOnce() -> Result<String>,
        PASSP: FnOnce() -> Result<Option<String>>,
    {
        // Generate keychain
        let mut keechain: KeeChain =
            KeeChain::generate(path, get_password, word_count, || Ok(None))?;
        let passphrase: Option<String> =
            get_passphrase().map_err(|e| Error::Generic(e.to_string()))?;
        if let Some(passphrase) = passphrase {
            keechain.keychain.add_passphrase(&passphrase);
            keechain.save()?;
            keechain.keychain.apply_passphrase(Some(passphrase));
        }

        // Get nostr keys
        let keys = Keys::from_mnemonic(
            keechain.keychain.seed.mnemonic().to_string(),
            keechain.keychain.seed.passphrase(),
        )?;

        Ok(Self {
            network,
            keechain,
            client: Client::new(&keys),
            endpoint: Arc::new(RwLock::new(None)),
            cache: Cache::new(),
        })
    }

    /// Restore keychain
    pub fn restore<P, PSW, M, PASSP>(
        path: P,
        get_password: PSW,
        get_mnemonic: M,
        get_passphrase: PASSP,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        PSW: FnOnce() -> Result<String>,
        M: FnOnce() -> Result<Mnemonic>,
        PASSP: FnOnce() -> Result<Option<String>>,
    {
        // Restore keychain
        let mut keechain: KeeChain = KeeChain::restore(path, get_password, get_mnemonic)?;
        let passphrase: Option<String> =
            get_passphrase().map_err(|e| Error::Generic(e.to_string()))?;
        if let Some(passphrase) = passphrase {
            keechain.keychain.add_passphrase(&passphrase);
            keechain.save()?;
            keechain.keychain.apply_passphrase(Some(passphrase));
        }

        // Get nostr keys
        let keys = Keys::from_mnemonic(
            keechain.keychain.seed.mnemonic().to_string(),
            keechain.keychain.seed.passphrase(),
        )?;

        Ok(Self {
            network,
            keechain,
            client: Client::new(&keys),
            endpoint: Arc::new(RwLock::new(None)),
            cache: Cache::new(),
        })
    }

    pub fn save(&self) -> Result<(), Error> {
        Ok(self.keechain.save()?)
    }

    pub fn check_password<S>(&self, password: S) -> bool
    where
        S: Into<String>,
    {
        self.keechain.check_password(password)
    }

    pub fn rename<P>(&mut self, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        Ok(self.keechain.rename(path)?)
    }

    pub fn change_password<NPSW>(&mut self, get_new_password: NPSW) -> Result<(), Error>
    where
        NPSW: FnOnce() -> Result<String>,
    {
        Ok(self.keechain.change_password(get_new_password)?)
    }

    pub fn wipe(&self) -> Result<(), Error> {
        Ok(self.keechain.wipe()?)
    }

    pub fn keychain(&self) -> Keychain {
        self.keechain.keychain.clone()
    }

    pub fn keys(&self) -> Keys {
        self.client.keys()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub async fn add_relay<S>(&self, url: S, proxy: Option<SocketAddr>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        Ok(self.client.add_relay(url, proxy).await?)
    }

    pub async fn connect(&self) {
        self.client.connect().await;
    }

    pub async fn add_relays_and_connect<S>(&self, relays: Vec<S>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let relays = relays.into_iter().map(|r| (r, None)).collect();
        self.client.add_relays(relays).await?;
        self.client.connect().await;
        Ok(())
    }

    pub async fn remove_relay<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        Ok(self.client.remove_relay(url).await?)
    }

    pub async fn relays(&self) -> HashMap<Url, Relay> {
        self.client.relays().await
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        Ok(self.client.shutdown().await?)
    }

    pub async fn set_electrum_endpoint<S>(&self, endpoint: S)
    where
        S: Into<String>,
    {
        let mut e = self.endpoint.write().await;
        *e = Some(endpoint.into());
    }

    pub async fn electrum_endpoint(&self) -> Result<String, Error> {
        let endpoint = self.endpoint.read().await;
        endpoint.clone().ok_or(Error::ElectrumEndpointNotSet)
    }

    pub fn wallet<S>(&self, descriptor: S) -> Result<Wallet<MemoryDatabase>, Error>
    where
        S: Into<String>,
    {
        let db = MemoryDatabase::new();
        Ok(Wallet::new(&descriptor.into(), None, self.network, db)?)
    }

    pub async fn get_contacts(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<XOnlyPublicKey, Metadata>, Error> {
        // TODO: get contacts from cache if `cache` feature enabled
        Ok(self.client.get_contact_list_metadata(timeout).await?)
    }

    pub async fn get_shared_keys(
        &self,
        timeout: Option<Duration>,
    ) -> Result<HashMap<EventId, Keys>, Error> {
        let keys = self.client.keys();

        let filter = Filter::new()
            .pubkey(keys.public_key())
            .kind(SHARED_KEY_KIND);
        let shared_key_events = self.client.get_events_of(vec![filter], timeout).await?;

        // Index global keys by policy id
        let mut shared_keys: HashMap<EventId, Keys> = HashMap::new();
        for event in shared_key_events.into_iter() {
            for tag in event.tags {
                if let Tag::Event(event_id, ..) = tag {
                    let content =
                        nips::nip04::decrypt(&keys.secret_key()?, &event.pubkey, &event.content)?;
                    let sk = SecretKey::from_str(&content)?;
                    let keys = Keys::new(sk);
                    shared_keys.insert(event_id, keys);
                }
            }
        }
        Ok(shared_keys)
    }

    pub async fn get_shared_key_by_policy_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<Keys, Error> {
        let keys = self.client.keys();

        let filter = Filter::new()
            .pubkey(keys.public_key())
            .event(policy_id)
            .kind(SHARED_KEY_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let shared_key_event = events.first().ok_or(Error::SharedKeysNotFound)?;
        let content = nips::nip04::decrypt(
            &keys.secret_key()?,
            &shared_key_event.pubkey,
            &shared_key_event.content,
        )?;
        let sk = SecretKey::from_str(&content)?;
        Ok(Keys::new(sk))
    }

    pub async fn get_policy_by_id(&self, policy_id: EventId) -> Result<Policy, Error> {
        self.cache
            .get_policy_by_id(policy_id)
            .await
            .ok_or(Error::PolicyNotFound)
    }

    pub async fn get_proposal_by_id(
        &self,
        proposal_id: EventId,
    ) -> Result<(EventId, Proposal), Error> {
        self.cache
            .get_proposal_by_id(proposal_id)
            .await
            .ok_or(Error::ProposalNotFound)
    }

    async fn get_approved_proposals_by_id(
        &self,
        proposal_id: EventId,
    ) -> Result<GetApprovedProposals, Error> {
        self.cache
            .get_approved_proposals_by_id(proposal_id)
            .await
            .ok_or(Error::ApprovedProposalNotFound)
    }

    pub async fn get_completed_proposal_by_id(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<(EventId, CompletedProposal), Error> {
        self.cache
            .get_completed_proposal_by_id(completed_proposal_id)
            .await
            .ok_or(Error::ProposalNotFound)
    }

    pub async fn delete_policy_by_id(
        &self,
        policy_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        // Get policy and shared keys
        let policy = self.get_policy_by_id(policy_id).await?;
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        // Extract pubkeys to notify users about proposal deletion
        let mut tags: Vec<Tag> = util::extract_public_keys(policy.descriptor.to_string())?
            .into_iter()
            .map(|p| Tag::PubKey(p, None))
            .collect();

        // Get all events linked to the policy
        let filter = Filter::new().event(policy_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?;

        tags.push(Tag::Event(policy_id, None, None));
        events
            .into_iter()
            .for_each(|e| tags.push(Tag::Event(e.id, None, None)));

        let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
        self.client.send_event(event).await?;

        self.cache.delete_policy_by_id(policy_id).await;

        Ok(())
    }

    pub async fn delete_proposal_by_id(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        // Get the proposal
        let filter = Filter::new().id(proposal_id).kind(PROPOSAL_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let proposal_event = events.first().ok_or(Error::ProposalNotFound)?;
        let policy_id =
            util::extract_first_event_id(proposal_event).ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        // Extract `p` tags from proposal event to notify users about proposal deletion
        let mut tags: Vec<Tag> = util::extract_tags_by_kind(proposal_event, TagKind::P)
            .into_iter()
            .cloned()
            .collect();

        // Get all events linked to the proposal
        /* let filter = Filter::new().event(proposal_id);
        let events = self.client.get_events_of(vec![filter], timeout).await?; */

        tags.push(Tag::Event(proposal_id, None, None));
        /* let mut ids: Vec<EventId> = vec![proposal_id];

        for event in events.into_iter() {
            if event.kind != COMPLETED_PROPOSAL_KIND {
                ids.push(event.id);
            }
        } */

        let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
        self.client.send_event(event).await?;

        self.cache.delete_proposal_by_id(proposal_id).await;

        Ok(())
    }

    pub async fn delete_completed_proposal_by_id(
        &self,
        completed_proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(), Error> {
        // Get the completed proposal
        let filter = Filter::new()
            .id(completed_proposal_id)
            .kind(COMPLETED_PROPOSAL_KIND);
        let events = self.client.get_events_of(vec![filter], timeout).await?;
        let proposal_event = events.first().ok_or(Error::ProposalNotFound)?;
        let policy_id = util::extract_tags_by_kind(proposal_event, TagKind::E)
            .get(1)
            .map(|t| {
                if let Tag::Event(event_id, ..) = t {
                    Some(event_id)
                } else {
                    None
                }
            })
            .ok_or(Error::PolicyNotFound)?
            .ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self
            .get_shared_key_by_policy_id(*policy_id, timeout)
            .await?;

        // Extract `p` tags from proposal event to notify users about proposal deletion
        let mut tags: Vec<Tag> = util::extract_tags_by_kind(proposal_event, TagKind::P)
            .into_iter()
            .cloned()
            .collect();

        tags.push(Tag::Event(completed_proposal_id, None, None));

        let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
        self.client.send_event(event).await?;

        self.cache
            .delete_completed_proposal_by_id(completed_proposal_id)
            .await;

        Ok(())
    }

    pub async fn get_policies(&self) -> BTreeMap<EventId, Policy> {
        self.cache.policies().await
    }

    pub async fn get_proposals(&self) -> BTreeMap<EventId, (EventId, Proposal)> {
        self.cache.proposals().await
    }

    pub async fn get_completed_proposals(&self) -> BTreeMap<EventId, (EventId, CompletedProposal)> {
        self.cache.completed_proposals().await
    }

    pub async fn save_policy<S>(
        &self,
        name: S,
        description: S,
        descriptor: S,
        // custom_pubkeys: Option<Vec<XOnlyPublicKey>>,
    ) -> Result<EventId, Error>
    where
        S: Into<String>,
    {
        let keys = self.client.keys();
        let descriptor = descriptor.into();

        /* let pubkeys = match custom_pubkeys {
            Some(pubkeys) => pubkeys,
            None => util::extract_public_keys(&descriptor)?
        }; */
        let extracted_pubkeys = util::extract_public_keys(&descriptor)?;

        // Generate a shared key
        let shared_key = Keys::generate();
        let policy = Policy::from_desc_or_policy(name, description, descriptor)?;

        // Compose the event
        let content = policy.encrypt(&shared_key)?;
        let tags: Vec<Tag> = extracted_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        // Publish policy with `shared_key` so every owner can delete it
        let policy_event = EventBuilder::new(POLICY_KIND, content, &tags).to_event(&shared_key)?;
        let policy_id = policy_event.id;

        // Publish the shared key
        for pubkey in extracted_pubkeys.into_iter() {
            let encrypted_shared_key = nips::nip04::encrypt(
                &keys.secret_key()?,
                &pubkey,
                shared_key.secret_key()?.display_secret().to_string(),
            )?;
            let event = EventBuilder::new(
                SHARED_KEY_KIND,
                encrypted_shared_key,
                &[Tag::Event(policy_id, None, None), Tag::PubKey(pubkey, None)],
            )
            .to_event(&keys)?;
            let event_id = self.client.send_event(event).await?;
            log::info!("Published shared key for {pubkey} at event {event_id}");
        }

        // Publish the event
        self.client.send_event(policy_event).await?;

        // Cache policy
        self.cache
            .cache_policy(policy_id, policy, self.network())
            .await?;

        Ok(policy_id)
    }

    pub async fn build_spending_proposal<S, B>(
        &self,
        policy: &Policy,
        to_address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        blockchain: &B,
    ) -> Result<(Proposal, TransactionDetails), Error>
    where
        S: Into<String>,
        B: Blockchain,
    {
        // Sync wallet
        let wallet = self.wallet(policy.descriptor.to_string())?;
        wallet.sync(blockchain, SyncOptions::default())?;

        // Get policies and specify which ones to use
        let wallet_policy = wallet
            .policies(KeychainKind::External)?
            .ok_or(Error::WalletSpendingPolicyNotFound)?;
        let mut path = BTreeMap::new();
        path.insert(wallet_policy.id, vec![1]);

        // Calculate fee rate
        let target_blocks: usize = fee_rate.target_blocks();
        let fee_rate = blockchain.estimate_fee(target_blocks)?;

        // Build the PSBT
        let (psbt, details) = {
            let mut builder = wallet.build_tx();
            builder
                .policy_path(path, KeychainKind::External)
                .fee_rate(fee_rate)
                .enable_rbf();
            match amount {
                Amount::Max => builder.drain_wallet().drain_to(to_address.script_pubkey()),
                Amount::Custom(amount) => builder.add_recipient(to_address.script_pubkey(), amount),
            };
            builder.finish()?
        };

        let amount: u64 = details.sent.saturating_sub(details.received);
        let proposal = Proposal::spending(to_address, amount, description, psbt);

        Ok((proposal, details))
    }

    /// Make a spending proposal
    pub async fn spend<S>(
        &self,
        policy_id: EventId,
        to_address: Address,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        timeout: Option<Duration>,
    ) -> Result<(EventId, Proposal), Error>
    where
        S: Into<String>,
    {
        // Get policy and shared keys
        let policy = self.get_policy_by_id(policy_id).await?;
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        let description: &str = &description.into();

        // Build spending proposal
        let endpoint = self.electrum_endpoint().await?;

        let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
        let (proposal, _details) = self
            .build_spending_proposal(
                &policy,
                to_address,
                amount,
                description,
                fee_rate,
                &blockchain,
            )
            .await?;

        if let Proposal::Spending {
            amount,
            description,
            ..
        } = &proposal
        {
            // Compose the event
            let extracted_pubkeys = util::extract_public_keys(policy.descriptor.to_string())?;
            let mut tags: Vec<Tag> = extracted_pubkeys
                .iter()
                .map(|p| Tag::PubKey(*p, None))
                .collect();
            tags.push(Tag::Event(policy_id, None, None));
            let content = proposal.encrypt(&shared_keys)?;
            // Publish proposal with `shared_key` so every owner can delete it
            let event = EventBuilder::new(PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;
            let proposal_id = self.client.send_event(event).await?;

            // Send DM msg
            let sender = self.client.keys().public_key();
            let mut msg = String::from("New spending proposal:\n");
            msg.push_str(&format!(
                "- Amount: {} sat\n",
                util::format::big_number(*amount)
            ));
            msg.push_str(&format!("- Description: {description}"));
            for pubkey in extracted_pubkeys.into_iter() {
                if sender != pubkey {
                    self.client.send_direct_msg(pubkey, &msg).await?;
                }
            }

            // Cache proposal
            self.cache
                .cache_proposal(proposal_id, policy_id, proposal.clone())
                .await;

            Ok((proposal_id, proposal))
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    fn is_internal_key<S>(&self, descriptor: S) -> Result<bool, Error>
    where
        S: Into<String>,
    {
        let descriptor = descriptor.into();
        let keys = self.client.keys();
        Ok(
            descriptor.starts_with(&format!("tr({}", keys.secret_key()?.public_key(SECP256K1)))
                || descriptor.starts_with(&format!("tr({}", keys.public_key())),
        )
    }

    pub fn approve_proposal(
        &self,
        policy: &Policy,
        proposal: &Proposal,
    ) -> Result<ApprovedProposal, Error> {
        let keys = self.client.keys();

        // Create a BDK wallet
        let mut wallet = self.wallet(policy.descriptor.to_string())?;

        // Add the BDK signer
        let private_key = PrivateKey::new(keys.secret_key()?, self.network);
        let signer = SignerWrapper::new(
            private_key,
            SignerContext::Tap {
                is_internal_key: self.is_internal_key(policy.descriptor.to_string())?,
            },
        );

        wallet.add_signer(KeychainKind::External, SignerOrdering(0), Arc::new(signer));

        // Sign the transaction
        let mut psbt = proposal.psbt();
        let _finalized = wallet.sign(&mut psbt, SignOptions::default())?;
        if psbt != proposal.psbt() {
            Ok(ApprovedProposal::new(psbt))
        } else {
            Err(Error::PsbtNotSigned)
        }
    }

    pub async fn approve(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(Event, ApprovedProposal), Error> {
        let keys = self.client.keys();

        // Get proposal
        let (policy_id, proposal) = self.get_proposal_by_id(proposal_id).await?;

        // Get policy
        let policy = self.get_policy_by_id(policy_id).await?;

        // Sign PSBT
        let approved_proposal = self.approve_proposal(&policy, &proposal)?;

        // Get shared keys
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        // Compose the event
        let content = approved_proposal.encrypt(&shared_keys)?;
        let extracted_pubkeys = util::extract_public_keys(policy.descriptor.to_string())?;
        let mut tags: Vec<Tag> = extracted_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        tags.push(Tag::Expiration(
            Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
        ));

        let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;

        // Publish the event
        self.client.send_event(event.clone()).await?;

        // Cache approved proposal
        self.cache
            .cache_approved_proposal(
                proposal_id,
                keys.public_key(),
                event.id,
                approved_proposal.psbt(),
                event.created_at,
            )
            .await;

        Ok((event, approved_proposal))
    }

    pub fn combine_psbts(
        &self,
        base_psbt: PartiallySignedTransaction,
        signed_psbts: Vec<PartiallySignedTransaction>,
    ) -> Result<PartiallySignedTransaction, Error> {
        let mut base_psbt = base_psbt;

        // Combine PSBTs
        for psbt in signed_psbts {
            base_psbt.combine(psbt)?;
        }

        // Finalize the transaction
        base_psbt
            .finalize_mut(SECP256K1)
            .map_err(Error::ImpossibleToFinalizePsbt)?;

        Ok(base_psbt)
    }

    pub fn combine_non_std_psbts(
        &self,
        policy: &Policy,
        base_psbt: PartiallySignedTransaction,
        signed_psbts: Vec<PartiallySignedTransaction>,
    ) -> Result<PartiallySignedTransaction, Error> {
        // Create a BDK wallet
        let wallet = self.wallet(policy.descriptor.to_string())?;

        let mut base_psbt = base_psbt;

        // Combine PSBTs
        for psbt in signed_psbts {
            base_psbt.combine(psbt)?;
        }

        // Finalize the transaction
        let signopts = SignOptions {
            trust_witness_utxo: true,
            remove_partial_sigs: false,
            ..Default::default()
        };
        if wallet.finalize_psbt(&mut base_psbt, signopts)? {
            Ok(base_psbt)
        } else {
            Err(Error::ImpossibleToFinalizeNonStdPsbt)
        }
    }

    pub async fn broadcast(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<Txid, Error> {
        // Get PSBTs
        let GetApprovedProposals {
            policy_id,
            proposal,
            signed_psbts,
            approvals,
        } = self.get_approved_proposals_by_id(proposal_id).await?;

        if let Proposal::Spending {
            description, psbt, ..
        } = proposal
        {
            let policy = self.get_policy_by_id(policy_id).await?;
            let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;
            let public_keys = util::extract_public_keys(policy.descriptor.to_string())?;

            // Combine PSBTs
            let psbt = self.combine_psbts(psbt, signed_psbts)?;
            let finalized_tx = psbt.extract_tx();

            // Broadcast
            let endpoint = self.electrum_endpoint().await?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            blockchain.broadcast(&finalized_tx)?;
            let txid = finalized_tx.txid();

            // Build the broadcasted proposal
            let completed_proposal = CompletedProposal::spending(txid, description, approvals);

            // Compose the event
            let content = completed_proposal.encrypt(&shared_keys)?;
            let mut tags: Vec<Tag> = public_keys.iter().map(|p| Tag::PubKey(*p, None)).collect();
            tags.push(Tag::Event(proposal_id, None, None));
            tags.push(Tag::Event(policy_id, None, None));
            let event = EventBuilder::new(COMPLETED_PROPOSAL_KIND, content, &tags)
                .to_event(&shared_keys)?;

            // Publish the event
            let event_id = self.client.send_event(event).await?;

            // Delete the proposal
            if let Err(e) = self.delete_proposal_by_id(proposal_id, timeout).await {
                log::error!("Impossibe to delete proposal {proposal_id}: {e}");
            }

            // Cache
            self.cache.delete_proposal_by_id(proposal_id).await;
            self.cache
                .sync_with_timechain(&blockchain, None, true)
                .await?;
            self.cache
                .cache_completed_proposal(event_id, policy_id, completed_proposal)
                .await;

            Ok(txid)
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    pub async fn build_proof_proposal<B, S>(
        &self,
        policy: &Policy,
        message: S,
        blockchain: &B,
    ) -> Result<Proposal, Error>
    where
        B: Blockchain,
        S: Into<String>,
    {
        let message: &str = &message.into();

        // Sync balance
        let wallet = self.wallet(policy.descriptor.to_string())?;
        wallet.sync(blockchain, SyncOptions::default())?;

        // Get policies and specify which ones to use
        let wallet_policy = wallet
            .policies(KeychainKind::External)?
            .ok_or(Error::WalletSpendingPolicyNotFound)?;
        let mut path = BTreeMap::new();
        path.insert(wallet_policy.id, vec![1]);

        let psbt: PartiallySignedTransaction = wallet.create_proof(message)?;

        Ok(Proposal::proof_of_reserve(message, psbt))
    }

    pub async fn new_proof_proposal<S>(
        &self,
        policy_id: EventId,
        message: S,
        timeout: Option<Duration>,
    ) -> Result<(EventId, Proposal, EventId), Error>
    where
        S: Into<String>,
    {
        let message: &str = &message.into();

        // Get policy and shared keys
        let policy = self.get_policy_by_id(policy_id).await?;
        let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;

        // Build proposal
        let endpoint = self.electrum_endpoint().await?;
        let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
        let proposal = self
            .build_proof_proposal(&policy, message, &blockchain)
            .await?;

        // Compose the event
        let extracted_pubkeys = util::extract_public_keys(policy.descriptor.to_string())?;
        let mut tags: Vec<Tag> = extracted_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(policy_id, None, None));
        let content = proposal.encrypt(&shared_keys)?;
        // Publish proposal with `shared_key` so every owner can delete it
        let event = EventBuilder::new(PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;
        let proposal_id = self.client.send_event(event).await?;

        // Send DM msg
        let sender = self.client.keys().public_key();
        let mut msg = String::from("New Proof of Reserve request:\n");
        msg.push_str(&format!("- Message: {message}"));
        for pubkey in extracted_pubkeys.into_iter() {
            if sender != pubkey {
                self.client.send_direct_msg(pubkey, &msg).await?;
            }
        }

        // Cache proposal
        self.cache
            .cache_proposal(proposal_id, policy_id, proposal.clone())
            .await;

        Ok((proposal_id, proposal, policy_id))
    }

    pub async fn finalize_proof(
        &self,
        proposal_id: EventId,
        timeout: Option<Duration>,
    ) -> Result<(EventId, CompletedProposal, EventId), Error> {
        // Get PSBTs
        let GetApprovedProposals {
            policy_id,
            proposal,
            signed_psbts,
            approvals,
        } = self.get_approved_proposals_by_id(proposal_id).await?;

        if let Proposal::ProofOfReserve { message, psbt } = proposal {
            let policy = self.get_policy_by_id(policy_id).await?;
            let shared_keys = self.get_shared_key_by_policy_id(policy_id, timeout).await?;
            let public_keys = util::extract_public_keys(policy.descriptor.to_string())?;

            // Combine PSBTs
            let psbt = self.combine_non_std_psbts(&policy, psbt, signed_psbts)?;

            // Build the completed proposal
            let completed_proposal =
                CompletedProposal::proof_of_reserve(message, policy.descriptor, psbt, approvals);

            // Compose the event
            let content = completed_proposal.encrypt(&shared_keys)?;
            let mut tags: Vec<Tag> = public_keys.iter().map(|p| Tag::PubKey(*p, None)).collect();
            tags.push(Tag::Event(proposal_id, None, None));
            tags.push(Tag::Event(policy_id, None, None));
            let event = EventBuilder::new(COMPLETED_PROPOSAL_KIND, content, &tags)
                .to_event(&shared_keys)?;

            // Publish the event
            let event_id = self.client.send_event(event).await?;

            // Delete the proposal
            if let Err(e) = self.delete_proposal_by_id(proposal_id, timeout).await {
                log::error!("Impossibe to delete proposal {proposal_id}: {e}");
            }

            // Cache
            self.cache.delete_proposal_by_id(proposal_id).await;
            self.cache
                .cache_completed_proposal(event_id, policy_id, completed_proposal.clone())
                .await;

            Ok((event_id, completed_proposal, policy_id))
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    pub async fn verify_proof(&self, proposal: CompletedProposal) -> Result<u64, Error> {
        if let CompletedProposal::ProofOfReserve {
            message,
            descriptor,
            psbt,
            ..
        } = proposal
        {
            let endpoint = self.electrum_endpoint().await?;
            let blockchain = ElectrumBlockchain::from(ElectrumClient::new(&endpoint)?);
            let wallet = self.wallet(descriptor.to_string())?;
            wallet.sync(&blockchain, SyncOptions::default())?;
            Ok(wallet.verify_proof(&psbt, message, None)?)
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    pub async fn verify_proof_by_id(&self, proposal_id: EventId) -> Result<u64, Error> {
        let (_policy_id, proposal) = self.get_completed_proposal_by_id(proposal_id).await?;
        self.verify_proof(proposal).await
    }
}

impl Coinstr {
    #[async_recursion::async_recursion]
    async fn wait_for_endpoint(&self) -> String {
        match self.electrum_endpoint().await {
            Ok(endpoint) => endpoint,
            Err(_) => {
                thread::sleep(Duration::from_secs(3)).await;
                self.wait_for_endpoint().await
            }
        }
    }

    fn sync_with_timechain(&self, sender: Sender<()>) -> AbortHandle {
        let this = self.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let timechain_sync = async move {
            let endpoint = this.wait_for_endpoint().await;
            let electrum_client = ElectrumClient::new(&endpoint).unwrap();
            let blockchain = ElectrumBlockchain::from(electrum_client);
            loop {
                if let Err(e) = this
                    .cache
                    .sync_with_timechain(&blockchain, Some(&sender), false)
                    .await
                {
                    log::error!("Impossible to sync wallets: {e}");
                }
                thread::sleep(Duration::from_secs(3)).await;
            }
        };

        let future = Abortable::new(timechain_sync, abort_registration);
        thread::spawn(async {
            let _ = future.await;
            log::debug!("Exited from wallet sync thread");
        });

        abort_handle
    }

    pub fn sync(&self) -> Receiver<()> {
        let (sender, receiver) = mpsc::channel(1024);
        let this = self.clone();
        thread::spawn(async move {
            // Sync timechain
            let abort_handle: AbortHandle = this.sync_with_timechain(sender.clone());

            let keys = this.client.keys();

            let shared_keys = this
                .get_shared_keys(Some(Duration::from_secs(60)))
                .await
                .unwrap_or_default();
            this.cache.cache_shared_keys(shared_keys).await;

            log::info!("Got shared keys");

            let filters = vec![
                Filter::new().pubkey(keys.public_key()).kinds(vec![
                    POLICY_KIND,
                    PROPOSAL_KIND,
                    COMPLETED_PROPOSAL_KIND,
                    Kind::EventDeletion,
                ]),
                Filter::new()
                    .pubkey(keys.public_key())
                    .kind(APPROVED_PROPOSAL_KIND)
                    .since(Timestamp::now() - APPROVED_PROPOSAL_EXPIRATION),
            ];

            this.client.subscribe(filters).await;
            let _ = this
                .client
                .handle_notifications(|notification| async {
                    match notification {
                        RelayPoolNotification::Event(_, event) => {
                            let event_id = event.id;
                            if event.is_expired() {
                                log::warn!("Event {event_id} expired");
                            } else {
                                match this.handle_event(event).await {
                                    Ok(_) => {
                                        sender.try_send(()).ok();
                                    }
                                    Err(e) => {
                                        log::error!("Impossible to handle event {event_id}: {e}");
                                    }
                                }
                            }
                        }
                        RelayPoolNotification::Shutdown => {
                            abort_handle.abort();
                        }
                        _ => (),
                    }

                    Ok(())
                })
                .await;
            log::debug!("Exited from nostr sync thread");
        });
        receiver
    }

    #[async_recursion::async_recursion]
    async fn handle_event(&self, event: Event) -> Result<()> {
        if event.kind == POLICY_KIND && !self.cache.policy_exists(event.id).await {
            if let Some(shared_key) = self.cache.get_shared_key_by_policy_id(event.id).await {
                let policy = Policy::decrypt(&shared_key, &event.content)?;
                self.cache
                    .cache_policy(event.id, policy, self.network())
                    .await?;
            } else {
                log::info!("Requesting shared key for {}", event.id);
                thread::sleep(Duration::from_secs(1)).await;
                let shared_key = self
                    .get_shared_key_by_policy_id(event.id, Some(Duration::from_secs(30)))
                    .await?;
                self.cache.cache_shared_key(event.id, shared_key).await;
                self.handle_event(event).await?;
            }
        } else if event.kind == PROPOSAL_KIND && !self.cache.proposal_exists(event.id).await {
            if let Some(policy_id) = util::extract_first_event_id(&event) {
                if let Some(shared_key) = self.cache.get_shared_key_by_policy_id(policy_id).await {
                    let proposal = Proposal::decrypt(&shared_key, &event.content)?;
                    self.cache
                        .cache_proposal(event.id, policy_id, proposal)
                        .await;
                } else {
                    log::info!("Requesting shared key for proposal {}", event.id);
                    thread::sleep(Duration::from_secs(1)).await;
                    let shared_key = self
                        .get_shared_key_by_policy_id(policy_id, Some(Duration::from_secs(30)))
                        .await?;
                    self.cache.cache_shared_key(policy_id, shared_key).await;
                    self.handle_event(event).await?;
                }
            } else {
                log::error!("Impossible to find policy id in proposal {}", event.id);
            }
        } else if event.kind == APPROVED_PROPOSAL_KIND {
            if let Some(proposal_id) = util::extract_first_event_id(&event) {
                if let Some(Tag::Event(policy_id, ..)) =
                    util::extract_tags_by_kind(&event, TagKind::E).get(1)
                {
                    if let Some(shared_key) =
                        self.cache.get_shared_key_by_policy_id(*policy_id).await
                    {
                        let approved_proposal =
                            ApprovedProposal::decrypt(&shared_key, &event.content)?;
                        self.cache
                            .cache_approved_proposal(
                                proposal_id,
                                event.pubkey,
                                event.id,
                                approved_proposal.psbt(),
                                event.created_at,
                            )
                            .await;
                    } else {
                        log::info!("Requesting shared key for approved proposal {}", event.id);
                        thread::sleep(Duration::from_secs(1)).await;
                        let shared_key = self
                            .get_shared_key_by_policy_id(*policy_id, Some(Duration::from_secs(30)))
                            .await?;
                        self.cache.cache_shared_key(*policy_id, shared_key).await;
                        self.handle_event(event).await?;
                    }
                } else {
                    log::error!("Impossible to find policy id in proposal {}", event.id);
                }
            } else {
                log::error!(
                    "Impossible to find proposal id in approved proposal {}",
                    event.id
                );
            }
        } else if event.kind == COMPLETED_PROPOSAL_KIND {
            if let Some(proposal_id) = util::extract_first_event_id(&event) {
                self.cache.delete_proposal_by_id(proposal_id).await;
                if let Some(Tag::Event(policy_id, ..)) =
                    util::extract_tags_by_kind(&event, TagKind::E).get(1)
                {
                    // Schedule policy for sync if the event was created in the last 60 secs
                    if event.created_at.add(Duration::from_secs(60)) >= Timestamp::now() {
                        self.cache.schedule_for_sync(*policy_id).await;
                    }

                    if let Some(shared_key) =
                        self.cache.get_shared_key_by_policy_id(*policy_id).await
                    {
                        let completed_proposal =
                            CompletedProposal::decrypt(&shared_key, &event.content)?;
                        self.cache
                            .cache_completed_proposal(event.id, *policy_id, completed_proposal)
                            .await;
                    } else {
                        log::info!("Requesting shared key for completed proposal {}", event.id);
                        thread::sleep(Duration::from_secs(1)).await;
                        let shared_key = self
                            .get_shared_key_by_policy_id(*policy_id, Some(Duration::from_secs(30)))
                            .await?;
                        self.cache.cache_shared_key(*policy_id, shared_key).await;
                        self.handle_event(event).await?;
                    }
                } else {
                    log::error!(
                        "Impossible to find policy id in completed proposal {}",
                        event.id
                    );
                }
            }
        } else if event.kind == Kind::EventDeletion {
            for tag in event.tags.iter() {
                if let Tag::Event(event_id, ..) = tag {
                    self.cache.uncache_generic_event_id(*event_id).await;
                }
            }
        }

        Ok(())
    }
}

impl Coinstr {
    #[allow(dead_code)]
    pub(crate) fn dummy(
        mnemonic: Mnemonic,
        passphrase: Option<&str>,
        network: Network,
    ) -> Result<Self, Error> {
        use keechain_core::types::keychain::EncryptionKeyType;
        let mut keechain: KeeChain = KeeChain::new(
            "./",
            "",
            1,
            EncryptionKeyType::Password,
            Keychain::new(mnemonic, Vec::new()),
        );
        keechain.keychain.apply_passphrase(passphrase);

        // Get nostr keys
        let keys = Keys::from_mnemonic(
            keechain.keychain.seed.mnemonic().to_string(),
            keechain.keychain.seed.passphrase(),
        )?;

        Ok(Self {
            network,
            keechain,
            client: Client::new(&keys),
            endpoint: Arc::new(RwLock::new(None)),
            cache: Cache::new(),
        })
    }
}

/* #[cfg(test)]
mod test {
    use bdk::blockchain::ElectrumBlockchain;
    use bdk::electrum_client::Client as ElectrumClient;

    use super::*;

    const NETWORK: Network = Network::Testnet;
    const BITCOIN_ENDPOINT: &str = "ssl://blockstream.info:993";

    #[tokio::test]
    async fn test_spend_approve_combine() -> Result<()> {
        let descriptor = "tr(38e977f65c9d4f7adafc50d7a181a5a4fcbbce3cda2f29bd123163e21e9bf307,multi_a(2,f831caf722214748c72db4829986bd0cbb2bb8b3aeade1c959624a52a9629046,3eea9e831fefdaa8df35187a204d82edb589a36b170955ac5ca6b88340befaa0))#39a2m6vn";

        let mnemonic_a = Mnemonic::from_str(
            "possible suffer flavor boring essay zoo collect stairs day cabbage wasp tackle",
        )?;
        let mnemonic_b = Mnemonic::from_str(
            "panther tree neglect narrow drip act visit position pass assault tennis long",
        )?;

        let client_a = Coinstr::dummy(mnemonic_a, None, NETWORK)?;
        let client_b = Coinstr::dummy(mnemonic_b, None, NETWORK)?;

        let policy = Policy::from_descriptor("Name", "Description", descriptor)?;

        // Build spending proposal
        let blockchain = ElectrumBlockchain::from(ElectrumClient::new(BITCOIN_ENDPOINT)?);
        let wallet = Wallet::new(
            &policy.descriptor.to_string(),
            None,
            NETWORK,
            MemoryDatabase::new(),
        )?;
        let (proposal, _) = client_a
            .build_spending_proposal(
                wallet,
                &blockchain,
                Address::from_str("mohjSavDdQYHRYXcS3uS6ttaHP8amyvX78")?,
                Amount::Custom(1120),
                "Testing",
                FeeRate::default(),
            )
            .await?;

        // Sign
        let approved_proposal_a = client_a.approve_proposal(&policy, &proposal)?;
        let approved_proposal_b = client_b.approve_proposal(&policy, &proposal)?;

        // Combine PSBTs
        let _tx = client_b.combine_psbts(
            proposal.psbt(),
            vec![approved_proposal_a.psbt(), approved_proposal_b.psbt()],
        )?;

        Ok(())
    }

    #[tokio::test]
    async fn test_proof_of_reserve() -> Result<()> {
        let descriptor = "tr(38e977f65c9d4f7adafc50d7a181a5a4fcbbce3cda2f29bd123163e21e9bf307,multi_a(2,f831caf722214748c72db4829986bd0cbb2bb8b3aeade1c959624a52a9629046,3eea9e831fefdaa8df35187a204d82edb589a36b170955ac5ca6b88340befaa0))#39a2m6vn";

        let mnemonic_a = Mnemonic::from_str(
            "possible suffer flavor boring essay zoo collect stairs day cabbage wasp tackle",
        )?;
        let mnemonic_b = Mnemonic::from_str(
            "panther tree neglect narrow drip act visit position pass assault tennis long",
        )?;

        let client_a = Coinstr::dummy(mnemonic_a, None, NETWORK)?;
        let client_b = Coinstr::dummy(mnemonic_b, None, NETWORK)?;

        let policy = Policy::from_descriptor("Name", "Description", descriptor)?;

        // Build spending proposal
        let blockchain = ElectrumBlockchain::from(ElectrumClient::new(BITCOIN_ENDPOINT)?);
        let proposal = client_a
            .build_proof_proposal(&policy, "Testing", &blockchain)
            .await?;

        // Sign
        let approved_proposal_a = client_a.approve_proposal(&policy, &proposal)?;
        let approved_proposal_b = client_b.approve_proposal(&policy, &proposal)?;

        // Combine PSBTs
        let _tx = client_b.combine_non_std_psbts(
            &policy,
            proposal.psbt(),
            vec![approved_proposal_a.psbt(), approved_proposal_b.psbt()],
        )?;

        Ok(())
    }
} */
