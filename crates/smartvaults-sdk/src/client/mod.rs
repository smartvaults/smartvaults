// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, HashMap, HashSet};
use std::net::SocketAddr;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use async_utility::thread;
use bdk_electrum::electrum_client::{
    Client as ElectrumClient, Config as ElectrumConfig, ElectrumApi, Socks5Config,
};
use nostr_sdk::nips::nip06::FromMnemonic;
use nostr_sdk::relay::pool;
use nostr_sdk::util::TryIntoUrl;
use nostr_sdk::{
    nips, Client, ClientMessage, Contact, Event, EventBuilder, EventId, Filter, Keys, Kind,
    Metadata, Options, Relay, RelayPoolNotification, Result, Tag, TagKind, Timestamp, UncheckedUrl,
    Url,
};
use parking_lot::RwLock as ParkingLotRwLock;
use smartvaults_core::bdk::chain::ConfirmationTime;
use smartvaults_core::bdk::signer::{SignerContext, SignerWrapper};
use smartvaults_core::bdk::wallet::{AddressIndex, Balance};
use smartvaults_core::bdk::FeeRate as BdkFeeRate;
use smartvaults_core::bips::bip39::Mnemonic;
use smartvaults_core::bitcoin::address::NetworkUnchecked;
use smartvaults_core::bitcoin::bip32::Fingerprint;
use smartvaults_core::bitcoin::hashes::sha256::Hash as Sha256Hash;
use smartvaults_core::bitcoin::hashes::Hash;
use smartvaults_core::bitcoin::psbt::PartiallySignedTransaction;
use smartvaults_core::bitcoin::{Address, Network, OutPoint, PrivateKey, ScriptBuf, Txid};
use smartvaults_core::secp256k1::XOnlyPublicKey;
use smartvaults_core::signer::smartvaults_signer;
use smartvaults_core::types::{KeeChain, Keychain, Seed, WordCount};
use smartvaults_core::{
    Amount, ApprovedProposal, CompletedProposal, FeeRate, Policy, PolicyTemplate, Proposal, Signer,
    SECP256K1,
};
use smartvaults_protocol::v1::constants::{
    APPROVED_PROPOSAL_EXPIRATION, APPROVED_PROPOSAL_KIND, COMPLETED_PROPOSAL_KIND, PROPOSAL_KIND,
    SHARED_KEY_KIND,
};
use smartvaults_protocol::v1::util::{Encryption, EncryptionError};
use smartvaults_protocol::v1::{
    Label, LabelData, SignerOffering, SmartVaultsEventBuilder, SmartVaultsEventBuilderError,
};
use smartvaults_sdk_sqlite::model::GetApprovalRaw;
use smartvaults_sdk_sqlite::Store;
use tokio::sync::broadcast::{self, Sender};
use tokio::sync::RwLock;

mod connect;
mod key_agent;
mod label;
mod signers;
mod sync;

pub use self::sync::{EventHandled, Message};
use crate::config::Config;
use crate::constants::{MAINNET_RELAYS, SEND_TIMEOUT, TESTNET_RELAYS};
use crate::manager::{Error as ManagerError, Manager, WalletError};
use crate::types::{
    GetAddress, GetApproval, GetApprovedProposals, GetCompletedProposal, GetPolicy, GetProposal,
    GetTransaction, GetUtxo, InternalGetPolicy, PolicyBackup, User,
};
use crate::util;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),
    #[error(transparent)]
    Keechain(#[from] smartvaults_core::types::keechain::Error),
    #[error(transparent)]
    Keychain(#[from] smartvaults_core::types::keychain::Error),
    #[error(transparent)]
    Dir(#[from] util::dir::Error),
    #[error(transparent)]
    Electrum(#[from] bdk_electrum::electrum_client::Error),
    #[error(transparent)]
    Url(#[from] nostr_sdk::url::ParseError),
    #[error(transparent)]
    Client(#[from] nostr_sdk::client::Error),
    #[error(transparent)]
    RelayPool(#[from] nostr_sdk::relay::pool::Error),
    #[error(transparent)]
    Keys(#[from] nostr_sdk::key::Error),
    #[error(transparent)]
    EventId(#[from] nostr_sdk::event::id::Error),
    #[error(transparent)]
    EventBuilder(#[from] nostr_sdk::event::builder::Error),
    #[error(transparent)]
    SmartVaultsEventBuilder(#[from] SmartVaultsEventBuilderError),
    #[error(transparent)]
    Relay(#[from] nostr_sdk::relay::Error),
    #[error(transparent)]
    Policy(#[from] smartvaults_core::policy::Error),
    #[error(transparent)]
    Proposal(#[from] smartvaults_core::proposal::Error),
    #[error(transparent)]
    Secp256k1(#[from] smartvaults_core::bitcoin::secp256k1::Error),
    #[error(transparent)]
    Address(#[from] smartvaults_core::bitcoin::address::Error),
    #[error(transparent)]
    Encryption(#[from] EncryptionError),
    #[error(transparent)]
    NIP04(#[from] nostr_sdk::nips::nip04::Error),
    #[error(transparent)]
    NIP06(#[from] nostr_sdk::nips::nip06::Error),
    #[error(transparent)]
    NIP46(#[from] nostr_sdk::nips::nip46::Error),
    #[error(transparent)]
    BIP32(#[from] smartvaults_core::bitcoin::bip32::Error),
    #[error(transparent)]
    Signer(#[from] smartvaults_core::signer::Error),
    #[error(transparent)]
    Manager(#[from] ManagerError),
    #[error(transparent)]
    Wallet(#[from] WalletError),
    #[error(transparent)]
    Config(#[from] crate::config::Error),
    #[error(transparent)]
    Store(#[from] smartvaults_sdk_sqlite::Error),
    #[error(transparent)]
    Label(#[from] smartvaults_protocol::v1::label::Error),
    #[error("password not match")]
    PasswordNotMatch,
    #[error("not enough public keys")]
    NotEnoughPublicKeys,
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
    #[error("signer not found")]
    SignerNotFound,
    #[error("signer ID not found")]
    SignerIdNotFound,
    #[error("public key not found")]
    PublicKeyNotFound,
    #[error("signer already shared")]
    SignerAlreadyShared,
    #[error("signer descriptor already exists")]
    SignerDescriptorAlreadyExists,
    #[error("nostr connect request already approved")]
    NostrConnectRequestAlreadyApproved,
    #[error("impossible to generate nostr connect response")]
    CantGenerateNostrConnectResponse,
    #[error("invalid fee rate")]
    InvalidFeeRate,
    #[error("impossible to delete a not owned event")]
    TryingToDeleteNotOwnedEvent,
    #[error("not found")]
    NotFound,
    #[error("{0}")]
    Generic(String),
}

/// Smart Vaults Client
#[derive(Debug, Clone)]
pub struct SmartVaults {
    network: Network,
    keechain: Arc<ParkingLotRwLock<KeeChain>>,
    client: Client,
    manager: Manager,
    config: Config,
    db: Store,
    syncing: Arc<AtomicBool>,
    sync_channel: Sender<Message>,
    default_signer: Signer,
    // Verified agents
    key_agents: Arc<RwLock<BTreeMap<XOnlyPublicKey, HashSet<SignerOffering>>>>,
}

impl SmartVaults {
    async fn new<P>(
        base_path: P,
        password: String,
        keechain: KeeChain,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
    {
        let base_path = base_path.as_ref();

        // Get nostr keys
        let seed = keechain.seed(password)?;
        let keys = Keys::from_mnemonic(seed.mnemonic().to_string(), seed.passphrase())?;

        // Open db
        let db = Store::open(
            util::dir::user_db(base_path, network, keys.public_key())?,
            &keys,
            network,
        )
        .await?;

        let opts = Options::new()
            .wait_for_connection(false)
            .wait_for_send(true)
            .wait_for_subscription(false)
            .skip_disconnected_relays(true)
            .send_timeout(Some(SEND_TIMEOUT));

        let (sender, _) = broadcast::channel::<Message>(4096);

        let this = Self {
            network,
            keechain: Arc::new(ParkingLotRwLock::new(keechain)),
            client: Client::with_opts(&keys, opts),
            manager: Manager::new(db.clone(), network),
            config: Config::try_from_file(base_path, network)?,
            db,
            syncing: Arc::new(AtomicBool::new(false)),
            sync_channel: sender,
            default_signer: smartvaults_signer(seed, network)?,
            key_agents: Arc::new(RwLock::new(BTreeMap::new())),
        };

        this.init().await?;

        Ok(this)
    }

    /// Open keychain
    pub async fn open<P, S>(
        base_path: P,
        name: S,
        password: S,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        S: Into<String>,
    {
        let base_path = base_path.as_ref();
        let password: String = password.into();

        // Open keychain
        let keychains_path: PathBuf = util::dir::keychains_path(base_path, network)?;
        let mut keechain: KeeChain = KeeChain::open(
            keychains_path,
            name,
            || Ok(password.clone()),
            network,
            &SECP256K1,
        )?;
        let passphrase: Option<String> = keechain.keychain(&password)?.get_passphrase(0);
        keechain.apply_passphrase(&password, passphrase, &SECP256K1)?;

        Self::new(base_path, password, keechain, network).await
    }

    /// Generate keychain
    pub async fn generate<P, S, PSW, CPSW, PASSP>(
        base_path: P,
        name: S,
        get_password: PSW,
        get_confirm_password: CPSW,
        word_count: WordCount,
        get_passphrase: PASSP,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        S: Into<String>,
        PSW: FnOnce() -> Result<String>,
        CPSW: FnOnce() -> Result<String>,
        PASSP: FnOnce() -> Result<Option<String>>,
    {
        let base_path = base_path.as_ref();

        let password: String = get_password().map_err(|e| Error::Generic(e.to_string()))?;

        // Generate keychain
        let keychains_path: PathBuf = util::dir::keychains_path(base_path, network)?;
        let mut keechain: KeeChain = KeeChain::generate(
            keychains_path,
            name,
            || Ok(password.clone()),
            get_confirm_password,
            word_count,
            || Ok(None),
            network,
            &SECP256K1,
        )?;
        let passphrase: Option<String> =
            get_passphrase().map_err(|e| Error::Generic(e.to_string()))?;
        if let Some(passphrase) = passphrase {
            keechain.add_passphrase(&password, &passphrase)?;
            keechain.save()?;
            keechain.apply_passphrase(&password, Some(passphrase), &SECP256K1)?;
        }

        Self::new(base_path, password, keechain, network).await
    }

    /// Restore keychain
    pub async fn restore<P, S, PSW, CPSW, M, PASSP>(
        base_path: P,
        name: S,
        get_password: PSW,
        get_confirm_password: CPSW,
        get_mnemonic: M,
        get_passphrase: PASSP,
        network: Network,
    ) -> Result<Self, Error>
    where
        P: AsRef<Path>,
        S: Into<String>,
        PSW: FnOnce() -> Result<String>,
        CPSW: FnOnce() -> Result<String>,
        M: FnOnce() -> Result<Mnemonic>,
        PASSP: FnOnce() -> Result<Option<String>>,
    {
        let base_path = base_path.as_ref();

        let password: String = get_password().map_err(|e| Error::Generic(e.to_string()))?;

        // Restore keychain
        let keychains_path: PathBuf = util::dir::keychains_path(base_path, network)?;
        let mut keechain: KeeChain = KeeChain::restore(
            keychains_path,
            name,
            || Ok(password.clone()),
            get_confirm_password,
            get_mnemonic,
            network,
            &SECP256K1,
        )?;
        let passphrase: Option<String> =
            get_passphrase().map_err(|e| Error::Generic(e.to_string()))?;
        if let Some(passphrase) = passphrase {
            keechain.add_passphrase(&password, &passphrase)?;
            keechain.save()?;
            keechain.apply_passphrase(&password, Some(passphrase), &SECP256K1)?;
        }

        Self::new(base_path, password, keechain, network).await
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub fn list_keychains<P>(base_path: P, network: Network) -> Result<Vec<String>, Error>
    where
        P: AsRef<Path>,
    {
        Ok(util::dir::get_keychains_list(base_path, network)?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    async fn init(&self) -> Result<(), Error> {
        for InternalGetPolicy {
            policy_id, policy, ..
        } in self.db.get_policies().await?.into_iter()
        {
            let manager = self.manager.clone();
            thread::spawn(async move {
                if let Err(e) = manager.load_policy(policy_id, policy).await {
                    tracing::error!("Impossible to load policy {policy_id}: {e}");
                }
            });
        }
        self.restore_relays().await?;
        self.client.connect().await;
        self.sync();
        Ok(())
    }

    /// Get keychain name
    pub fn name(&self) -> Option<String> {
        self.keechain.read().name()
    }

    /// Check keychain password
    pub fn check_password<T>(&self, password: T) -> bool
    where
        T: AsRef<[u8]>,
    {
        self.keechain.read().check_password(password)
    }

    /// Rename keychain file
    pub fn rename<S>(&self, new_name: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let mut keechain = self.keechain.write();
        Ok(keechain.rename(new_name)?)
    }

    /// Change keychain password
    pub fn change_password<PSW, NPSW, NCPSW>(
        &self,
        get_old_password: PSW,
        get_new_password: NPSW,
        get_new_confirm_password: NCPSW,
    ) -> Result<(), Error>
    where
        PSW: FnOnce() -> Result<String>,
        NPSW: FnOnce() -> Result<String>,
        NCPSW: FnOnce() -> Result<String>,
    {
        let mut keechain = self.keechain.write();
        Ok(keechain.change_password(
            get_old_password,
            get_new_password,
            get_new_confirm_password,
        )?)
    }

    /// Permanent delete the keychain
    pub fn wipe<T>(&self, password: T) -> Result<(), Error>
    where
        T: AsRef<[u8]>,
    {
        if self.check_password(password) {
            Ok(self.keechain.read().wipe()?)
        } else {
            Err(Error::PasswordNotMatch)
        }
    }

    pub async fn start(&self) {
        self.client.start().await;
        self.sync();
    }

    pub async fn stop(&self) -> Result<(), Error> {
        self.client.stop().await?;
        Ok(())
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<(), Error> {
        self.client.stop().await?;
        self.client
            .handle_notifications(|notification: RelayPoolNotification| async move {
                if let RelayPoolNotification::Stop = notification {
                    self.db.wipe().await?;
                    self.manager.unload_policies().await;
                    self.client.clear_already_seen_events().await;
                    self.client.start().await;
                    self.sync();
                }
                Ok(false)
            })
            .await?;
        Ok(())
    }

    pub fn keychain<T>(&self, password: T) -> Result<Keychain, Error>
    where
        T: AsRef<[u8]>,
    {
        Ok(self.keechain.read().keychain(password)?)
    }

    pub async fn keys(&self) -> Keys {
        self.client.keys().await
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.keechain.read().fingerprint()
    }

    pub fn network(&self) -> Network {
        self.network
    }

    pub async fn add_relay<S>(&self, url: S, proxy: Option<SocketAddr>) -> Result<(), Error>
    where
        S: Into<String>,
    {
        self.add_relay_with_opts(url, proxy, true).await
    }

    pub async fn add_relay_with_opts<S>(
        &self,
        url: S,
        proxy: Option<SocketAddr>,
        save_to_relay_list: bool,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let url = Url::parse(&url.into())?;
        self.db.insert_relay(url.clone(), proxy).await?;
        self.db.enable_relay(url.clone()).await?;

        if self.client.add_relay(url.as_str(), proxy).await? {
            let relay = self.client.relay(&url).await?;
            let last_sync: Timestamp = match self.db.get_last_relay_sync(url.clone()).await {
                Ok(ts) => ts,
                Err(_) => Timestamp::from(0),
            };
            let filters: Vec<Filter> = self.sync_filters(last_sync).await;
            relay.subscribe(filters, None).await?;
            relay.connect(false).await;

            if save_to_relay_list {
                let this = self.clone();
                thread::spawn(async move {
                    if let Err(e) = this.save_relay_list().await {
                        tracing::error!("Impossible to save relay list: {e}");
                    }
                });
            }

            if let Err(e) = self.rebroadcast_to(url.clone()).await {
                tracing::error!("Impossible to rebroadcast events to {url}: {e}");
            }
        }

        Ok(())
    }

    /// Save relay list (NIP65)
    pub async fn save_relay_list(&self) -> Result<EventId, Error> {
        let keys = self.keys().await;
        let relays = self.client.relays().await;
        let list = relays
            .into_keys()
            .map(|url| (UncheckedUrl::from(url), None))
            .collect();
        let event = EventBuilder::relay_list(list).to_event(&keys)?;
        self.send_event(event).await
    }

    /// Get default relays for current [`Network`]
    pub fn default_relays(&self) -> Vec<String> {
        match self.network {
            Network::Bitcoin => MAINNET_RELAYS.into_iter().map(|r| r.to_string()).collect(),
            _ => TESTNET_RELAYS.into_iter().map(|r| r.to_string()).collect(),
        }
    }

    async fn load_nostr_connect_relays(&self) -> Result<(), Error> {
        let relays = self.db.get_nostr_connect_sessions_relays().await?;
        let relays = relays.into_iter().map(|r| (r, None)).collect();
        self.client.add_relays(relays).await?;
        Ok(())
    }

    /// Restore relays
    #[tracing::instrument(skip_all, level = "trace")]
    async fn restore_relays(&self) -> Result<(), Error> {
        let relays = self.db.get_relays(true).await?;
        for (url, proxy) in relays.into_iter() {
            self.client.add_relay(url, proxy).await?;
        }

        if self.client.relays().await.is_empty() {
            for url in self.default_relays().into_iter() {
                let url = Url::parse(&url)?;
                self.client.add_relay(&url, None).await?;
                self.db.insert_relay(url.clone(), None).await?;
                self.db.enable_relay(url).await?;
            }
        }

        // Restore Nostr Connect Session relays
        self.load_nostr_connect_relays().await?;

        Ok(())
    }

    pub async fn remove_relay<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        self.remove_relay_with_opts(url, true).await
    }

    pub async fn remove_relay_with_opts<S>(
        &self,
        url: S,
        save_to_relay_list: bool,
    ) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let url = Url::parse(&url.into())?;
        self.db.delete_relay(url.clone()).await?;
        if save_to_relay_list {
            if let Err(e) = self.save_relay_list().await {
                tracing::error!("Impossible to save relay list: {e}");
            }
        }
        Ok(self.client.remove_relay(url).await?)
    }

    pub async fn connect_relay<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let url = Url::parse(&url.into())?;
        self.db.enable_relay(url.clone()).await?;
        self.client.connect_relay(url).await?;
        Ok(())
    }

    pub async fn disconnect_relay<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let url = Url::parse(&url.into())?;
        self.db.disable_relay(url.clone()).await?;
        self.client.disconnect_relay(url).await?;
        Ok(())
    }

    pub async fn relays(&self) -> BTreeMap<Url, Relay> {
        self.client.relays().await.into_iter().collect()
    }

    pub async fn relay<U>(&self, url: U) -> Result<Relay, Error>
    where
        U: TryIntoUrl,
        pool::Error: From<<U as TryIntoUrl>::Err>,
    {
        Ok(self.client.relay(url).await?)
    }

    pub async fn shutdown(self) -> Result<(), Error> {
        self.manager.unload_policies().await;
        Ok(self.client.shutdown().await?)
    }

    async fn send_event(&self, event: Event) -> Result<EventId, Error> {
        self.db.save_event(event.clone()).await?;
        Ok(self.client.send_event(event).await?)
    }

    /// Get config
    pub fn config(&self) -> Config {
        self.config.clone()
    }

    pub async fn set_electrum_endpoint<S>(&self, endpoint: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        // Set electrum endpoint
        self.config.set_electrum_endpoint(Some(endpoint)).await;
        // Save config file
        self.config.save().await?;
        Ok(())
    }

    pub async fn electrum_endpoint(&self) -> Result<String, Error> {
        Ok(self.config.electrum_endpoint().await?)
    }

    pub fn block_height(&self) -> u32 {
        self.manager.block_height()
    }

    pub async fn set_metadata(&self, metadata: Metadata) -> Result<(), Error> {
        let keys: Keys = self.keys().await;
        let event = EventBuilder::set_metadata(metadata.clone()).to_event(&keys)?;
        self.send_event(event).await?;
        self.db.set_metadata(keys.public_key(), metadata).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_profile(&self) -> Result<User, Error> {
        let public_key: XOnlyPublicKey = self.keys().await.public_key();
        let metadata = self.db.get_metadata(public_key).await?;
        Ok(User::new(public_key, metadata))
    }

    /// Get [`Metadata`] of [`XOnlyPublicKey`]
    ///
    /// If not exists in local database, will return an empty [`Metadata`] struct and will request
    /// it to relays
    pub async fn get_public_key_metadata(
        &self,
        public_key: XOnlyPublicKey,
    ) -> Result<Metadata, Error> {
        let metadata: Metadata = self.db.get_metadata(public_key).await?;
        if metadata == Metadata::default() {
            self.client
                .req_events_of(
                    vec![Filter::new()
                        .author(public_key.to_string())
                        .kind(Kind::Metadata)
                        .limit(1)],
                    Some(Duration::from_secs(60)),
                )
                .await;
        }
        Ok(metadata)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_contacts(&self) -> Result<Vec<User>, Error> {
        Ok(self
            .db
            .get_contacts_with_metadata()
            .await?
            .into_iter()
            .map(|(public_key, metadata)| User::new(public_key, metadata))
            .collect())
    }

    pub async fn add_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        let keys: Keys = self.keys().await;
        if public_key != keys.public_key() {
            // Add contact
            let mut contacts: Vec<Contact> = self
                .db
                .get_contacts_public_keys()
                .await?
                .into_iter()
                .map(|p| Contact::new::<String>(p, None, None))
                .collect();
            contacts.push(Contact::new::<String>(public_key, None, None));
            let event = EventBuilder::set_contact_list(contacts).to_event(&keys)?;
            self.send_event(event).await?;
            self.db.save_contact(public_key).await?;

            // Request contact metadata
            self.client
                .req_events_of(
                    vec![Filter::new()
                        .author(public_key.to_string())
                        .kind(Kind::Metadata)
                        .limit(1)],
                    Some(Duration::from_secs(60)),
                )
                .await;
        }

        Ok(())
    }

    pub async fn remove_contact(&self, public_key: XOnlyPublicKey) -> Result<(), Error> {
        let keys: Keys = self.keys().await;
        let contacts: Vec<Contact> = self
            .db
            .get_contacts_public_keys()
            .await?
            .into_iter()
            .filter(|p| p != &public_key)
            .map(|p| Contact::new::<String>(p, None, None))
            .collect();
        let event = EventBuilder::set_contact_list(contacts).to_event(&keys)?;
        self.send_event(event).await?;
        self.db.delete_contact(public_key).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_policy_by_id(&self, policy_id: EventId) -> Result<GetPolicy, Error> {
        let InternalGetPolicy {
            policy_id,
            policy,
            last_sync,
        } = self.db.get_policy(policy_id).await?;
        Ok(GetPolicy {
            policy_id,
            policy,
            balance: self.manager.get_balance(policy_id).await?,
            last_sync,
        })
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_proposal_by_id(&self, proposal_id: EventId) -> Result<GetProposal, Error> {
        Ok(self.db.get_proposal(proposal_id).await?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_completed_proposal_by_id(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<GetCompletedProposal, Error> {
        Ok(self
            .db
            .get_completed_proposal(completed_proposal_id)
            .await?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn delete_policy_by_id(&self, policy_id: EventId) -> Result<(), Error> {
        let Event { pubkey, .. } = self.db.get_event_by_id(policy_id).await?;

        // Get nostr pubkeys and shared keys
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;
        let shared_keys: Keys = self.db.get_shared_key(policy_id).await?;

        if pubkey == shared_keys.public_key() {
            // Get all events linked to the policy
            let event_ids = self.db.get_event_ids_linked_to_policy(policy_id).await?;

            let mut tags: Vec<Tag> = nostr_pubkeys
                .into_iter()
                .map(|p| Tag::PubKey(p, None))
                .collect();
            tags.push(Tag::Event(policy_id, None, None));
            event_ids
                .into_iter()
                .for_each(|id| tags.push(Tag::Event(id, None, None)));

            let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
            self.send_event(event).await?;

            self.db.delete_policy(policy_id).await?;

            self.manager.unload_policy(policy_id).await?;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    pub async fn delete_proposal_by_id(&self, proposal_id: EventId) -> Result<(), Error> {
        // Get the proposal
        let proposal_event = self.db.get_event_by_id(proposal_id).await?;
        if proposal_event.kind != PROPOSAL_KIND {
            return Err(Error::ProposalNotFound);
        }

        let policy_id =
            util::extract_first_event_id(&proposal_event).ok_or(Error::PolicyNotFound)?;

        // Get shared key
        let shared_keys = self.db.get_shared_key(policy_id).await?;

        if proposal_event.pubkey == shared_keys.public_key() {
            // Extract `p` tags from proposal event to notify users about proposal deletion
            let mut tags: Vec<Tag> = util::extract_tags_by_kind(&proposal_event, TagKind::P)
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
            self.send_event(event).await?;

            self.db.delete_proposal(proposal_id).await?;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    pub async fn delete_completed_proposal_by_id(
        &self,
        completed_proposal_id: EventId,
    ) -> Result<(), Error> {
        // Get the completed proposal
        let proposal_event = self.db.get_event_by_id(completed_proposal_id).await?;
        if proposal_event.kind != COMPLETED_PROPOSAL_KIND {
            return Err(Error::ProposalNotFound);
        }

        let policy_id = util::extract_tags_by_kind(&proposal_event, TagKind::E)
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
        let shared_keys = self.db.get_shared_key(*policy_id).await?;

        if proposal_event.pubkey == shared_keys.public_key() {
            // Extract `p` tags from proposal event to notify users about proposal deletion
            let mut tags: Vec<Tag> = util::extract_tags_by_kind(&proposal_event, TagKind::P)
                .into_iter()
                .cloned()
                .collect();

            tags.push(Tag::Event(completed_proposal_id, None, None));

            let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&shared_keys)?;
            self.send_event(event).await?;

            self.db
                .delete_completed_proposal(completed_proposal_id)
                .await?;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_policies(&self) -> Result<Vec<GetPolicy>, Error> {
        let mut policies = Vec::new();

        for InternalGetPolicy {
            policy_id,
            policy,
            last_sync,
        } in self.db.get_policies().await?.into_iter()
        {
            policies.push(GetPolicy {
                policy_id,
                policy,
                balance: self.manager.get_balance(policy_id).await?,
                last_sync,
            });
        }

        Ok(policies)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_proposals(&self) -> Result<Vec<GetProposal>, Error> {
        Ok(self.db.get_proposals().await?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_proposals_by_policy_id(
        &self,
        policy_id: EventId,
    ) -> Result<Vec<GetProposal>, Error> {
        Ok(self.db.get_proposals_by_policy_id(policy_id).await?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_approvals_by_proposal_id(
        &self,
        proposal_id: EventId,
    ) -> Result<Vec<GetApproval>, Error> {
        let mut list = Vec::new();
        let approvals = self.db.get_approvals_by_proposal_id(proposal_id).await?;
        for GetApprovalRaw {
            approval_id,
            public_key,
            approved_proposal,
            timestamp,
        } in approvals.into_iter()
        {
            let metadata: Metadata = self.db.get_metadata(public_key).await?;
            list.push(GetApproval {
                approval_id,
                user: User::new(public_key, metadata),
                approved_proposal,
                timestamp,
            });
        }
        Ok(list)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_completed_proposals(&self) -> Result<Vec<GetCompletedProposal>, Error> {
        Ok(self.db.completed_proposals().await?)
    }

    pub async fn save_policy<S>(
        &self,
        name: S,
        description: S,
        descriptor: S,
        nostr_pubkeys: Vec<XOnlyPublicKey>,
    ) -> Result<EventId, Error>
    where
        S: Into<String>,
    {
        let keys: Keys = self.keys().await;
        let descriptor = descriptor.into();

        if nostr_pubkeys.is_empty() {
            return Err(Error::NotEnoughPublicKeys);
        }

        // Generate a shared key
        let shared_key = Keys::generate();
        let policy = Policy::from_desc_or_policy(name, description, descriptor, self.network)?;

        // Compose the event
        // Publish it with `shared_key` so every owner can delete it
        let policy_event: Event = EventBuilder::policy(&shared_key, &policy, &nostr_pubkeys)?;
        let policy_id = policy_event.id;

        // Publish the shared key
        for pubkey in nostr_pubkeys.iter() {
            let event: Event = EventBuilder::shared_key(&keys, &shared_key, pubkey, policy_id)?;
            let event_id: EventId = event.id;

            // TODO: use send_batch_event method from nostr-sdk
            self.db.save_event(event.clone()).await?;
            self.client
                .pool()
                .send_msg(ClientMessage::new_event(event), None)
                .await?;
            tracing::info!("Published shared key for {pubkey} at event {event_id}");
        }

        // Publish the event
        self.send_event(policy_event).await?;

        // Cache policy
        self.db.save_shared_key(policy_id, shared_key).await?;
        self.db
            .save_policy(policy_id, policy.clone(), nostr_pubkeys)
            .await?;

        // Load policy
        self.manager.load_policy(policy_id, policy).await?;

        Ok(policy_id)
    }

    pub async fn save_policy_from_template<S>(
        &self,
        name: S,
        description: S,
        template: PolicyTemplate,
        nostr_pubkeys: Vec<XOnlyPublicKey>,
    ) -> Result<EventId, Error>
    where
        S: Into<String>,
    {
        let policy: Policy = Policy::from_template(name, description, template, self.network)?;
        self.save_policy(
            policy.name,
            policy.description,
            policy.descriptor.to_string(),
            nostr_pubkeys,
        )
        .await
    }

    /// Make a spending proposal
    #[allow(clippy::too_many_arguments)]
    pub async fn spend<S>(
        &self,
        policy_id: EventId,
        address: Address<NetworkUnchecked>,
        amount: Amount,
        description: S,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
        skip_frozen_utxos: bool,
    ) -> Result<GetProposal, Error>
    where
        S: Into<String>,
    {
        let description: &str = &description.into();

        // Check and calculate fee rate
        if !fee_rate.is_valid() {
            return Err(Error::InvalidFeeRate);
        }

        let fee_rate: BdkFeeRate = match fee_rate {
            FeeRate::Priority(priority) => {
                let endpoint = self.config.electrum_endpoint().await?;
                let proxy: Option<SocketAddr> = self.config.proxy().await.ok();
                let config = ElectrumConfig::builder()
                    .socks5(proxy.map(Socks5Config::new))
                    .build();
                let blockchain = ElectrumClient::from_config(&endpoint, config)?;
                let btc_per_kvb: f32 =
                    blockchain.estimate_fee(priority.target_blocks() as usize)? as f32;
                BdkFeeRate::from_btc_per_kvb(btc_per_kvb)
            }
            FeeRate::Rate(rate) => BdkFeeRate::from_sat_per_vb(rate),
        };

        let mut frozen_utxos: Option<Vec<OutPoint>> = None;
        if !skip_frozen_utxos {
            let mut list = Vec::new();
            let hashed_frozen_utxos = self.db.get_frozen_utxos(policy_id).await?;
            for local_utxo in self.manager.get_utxos(policy_id).await?.into_iter() {
                let hash = Sha256Hash::hash(local_utxo.outpoint.to_string().as_bytes());
                if hashed_frozen_utxos.contains(&hash) {
                    list.push(local_utxo.outpoint);
                }
            }
            frozen_utxos = Some(list);
        }

        // Build spending proposal
        let proposal: Proposal = self
            .manager
            .spend(
                policy_id,
                address,
                amount,
                description,
                fee_rate,
                utxos,
                frozen_utxos,
                policy_path,
            )
            .await?;

        if let Proposal::Spending { .. } = &proposal {
            // Get shared keys
            let shared_key: Keys = self.db.get_shared_key(policy_id).await?;

            // Compose the event
            let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;
            let event: Event =
                EventBuilder::proposal(&shared_key, policy_id, &proposal, &nostr_pubkeys)?;
            let proposal_id = self.send_event(event).await?;

            // Send DM msg
            // TODO: send withoud wait for OK
            /* let sender = self.client.keys().public_key();
            let mut msg = String::from("New spending proposal:\n");
            msg.push_str(&format!(
                "- Amount: {} sat\n",
                util::format::big_number(*amount)
            ));
            msg.push_str(&format!("- Description: {description}"));
            for pubkey in nostr_pubkeys.into_iter() {
                if sender != pubkey {
                    self.client.send_direct_msg(pubkey, &msg, None).await?;
                }
            } */

            // Cache proposal
            self.db
                .save_proposal(proposal_id, policy_id, proposal.clone())
                .await?;

            Ok(GetProposal {
                proposal_id,
                policy_id,
                proposal,
                signed: false,
            })
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    /// Spend to another [`Policy`]
    #[allow(clippy::too_many_arguments)]
    pub async fn self_transfer(
        &self,
        from_policy_id: EventId,
        to_policy_id: EventId,
        amount: Amount,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
        skip_frozen_utxos: bool,
    ) -> Result<GetProposal, Error> {
        let address = self
            .get_address(to_policy_id, AddressIndex::New)
            .await?
            .address;
        let description: String = format!(
            "Self transfer from policy #{} to #{}",
            util::cut_event_id(from_policy_id),
            util::cut_event_id(to_policy_id)
        );
        self.spend(
            from_policy_id,
            Address::new(self.network, address.payload),
            amount,
            description,
            fee_rate,
            utxos,
            policy_path,
            skip_frozen_utxos,
        )
        .await
    }

    async fn is_internal_key<S>(&self, descriptor: S) -> Result<bool, Error>
    where
        S: Into<String>,
    {
        let descriptor = descriptor.into();
        let keys: Keys = self.keys().await;
        Ok(
            descriptor.starts_with(&format!("tr({}", keys.normalized_public_key()?))
                || descriptor.starts_with(&format!("tr({}", keys.public_key())),
        )
    }

    pub async fn approve<T>(
        &self,
        password: T,
        proposal_id: EventId,
    ) -> Result<(EventId, ApprovedProposal), Error>
    where
        T: AsRef<[u8]>,
    {
        // Get proposal and policy
        let GetProposal {
            policy_id,
            proposal,
            ..
        } = self.get_proposal_by_id(proposal_id).await?;
        let GetPolicy { policy, .. } = self.get_policy_by_id(policy_id).await?;

        // Sign PSBT
        // Custom signer
        let keys: Keys = self.keys().await;
        let signer = SignerWrapper::new(
            PrivateKey::new(keys.secret_key()?, self.network),
            SignerContext::Tap {
                is_internal_key: self.is_internal_key(policy.descriptor.to_string()).await?,
            },
        );
        let seed: Seed = self.keechain.read().seed(password)?;
        let approved_proposal = proposal.approve(&seed, vec![signer], self.network)?;

        // Get shared keys
        let shared_keys: Keys = self.db.get_shared_key(policy_id).await?;

        // Compose the event
        let content = approved_proposal.encrypt_with_keys(&shared_keys)?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .into_iter()
            .map(|p| Tag::PubKey(p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        tags.push(Tag::Expiration(
            Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
        ));

        let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;
        let timestamp = event.created_at;

        // Publish the event
        let event_id = self.send_event(event).await?;

        // Cache approved proposal
        self.db
            .save_approved_proposal(
                proposal_id,
                keys.public_key(),
                event_id,
                approved_proposal.clone(),
                timestamp,
            )
            .await?;

        Ok((event_id, approved_proposal))
    }

    pub async fn approve_with_signed_psbt(
        &self,
        proposal_id: EventId,
        signed_psbt: PartiallySignedTransaction,
    ) -> Result<(EventId, ApprovedProposal), Error> {
        let keys: Keys = self.keys().await;

        // Get proposal and policy
        let GetProposal {
            policy_id,
            proposal,
            ..
        } = self.get_proposal_by_id(proposal_id).await?;

        let approved_proposal = proposal.approve_with_signed_psbt(signed_psbt)?;

        // Get shared keys
        let shared_keys: Keys = self.db.get_shared_key(policy_id).await?;

        // Compose the event
        let content = approved_proposal.encrypt_with_keys(&shared_keys)?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .into_iter()
            .map(|p| Tag::PubKey(p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        tags.push(Tag::Expiration(
            Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
        ));

        let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;
        let timestamp = event.created_at;

        // Publish the event
        let event_id = self.send_event(event).await?;

        // Cache approved proposal
        self.db
            .save_approved_proposal(
                proposal_id,
                keys.public_key(),
                event_id,
                approved_proposal.clone(),
                timestamp,
            )
            .await?;

        Ok((event_id, approved_proposal))
    }

    /* pub async fn approve_with_hwi_signer(
        &self,
        proposal_id: EventId,
        signer: Signer,
    ) -> Result<(EventId, ApprovedProposal), Error> {
        let keys: Keys = self.keys().await;

        // Get proposal and policy
        let GetProposal {
            policy_id,
            proposal,
            ..
        } = self.get_proposal_by_id(proposal_id)?;

        let approved_proposal = proposal.approve_with_hwi_signer(signer, self.network)?;

        // Get shared keys
        let shared_keys: Keys = self.db.get_shared_key(policy_id).await?;

        // Compose the event
        let content = approved_proposal.encrypt_with_keys(&shared_keys)?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .into_iter()
            .map(|p| Tag::PubKey(p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        tags.push(Tag::Expiration(
            Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
        ));

        let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;
        let timestamp = event.created_at;

        // Publish the event
        let event_id = self.send_event(event).await?;

        // Cache approved proposal
        self.db.save_approved_proposal(
            proposal_id,
            keys.public_key(),
            event_id,
            approved_proposal.clone(),
            timestamp,
        )?;

        Ok((event_id, approved_proposal))
    } */

    pub async fn revoke_approval(&self, approval_id: EventId) -> Result<(), Error> {
        let Event { pubkey, .. } = self.db.get_event_by_id(approval_id).await?;
        let keys: Keys = self.keys().await;
        if pubkey == keys.public_key() {
            let policy_id = self.db.get_policy_id_by_approval_id(approval_id).await?;

            // Get nostr pubkeys linked to policy
            let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;

            let mut tags: Vec<Tag> = nostr_pubkeys
                .into_iter()
                .map(|p| Tag::PubKey(p, None))
                .collect();
            tags.push(Tag::Event(approval_id, None, None));

            let event = EventBuilder::new(Kind::EventDeletion, "", &tags).to_event(&keys)?;
            self.send_event(event).await?;

            self.db.delete_approval(approval_id).await?;

            Ok(())
        } else {
            Err(Error::TryingToDeleteNotOwnedEvent)
        }
    }

    /// Finalize [`Proposal`]
    pub async fn finalize(&self, proposal_id: EventId) -> Result<CompletedProposal, Error> {
        // Get PSBTs
        let GetApprovedProposals {
            policy_id,
            proposal,
            approved_proposals,
        } = self.db.get_approved_proposals_by_id(proposal_id).await?;

        let shared_keys = self.db.get_shared_key(policy_id).await?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;

        // Finalize proposal
        let completed_proposal: CompletedProposal =
            proposal.finalize(approved_proposals, self.network)?;

        // Broadcast
        if let CompletedProposal::Spending { tx, .. } = &completed_proposal {
            let endpoint = self.config.electrum_endpoint().await?;
            let proxy: Option<SocketAddr> = self.config.proxy().await.ok();
            let config = ElectrumConfig::builder()
                .socks5(proxy.map(Socks5Config::new))
                .build();
            let blockchain = ElectrumClient::from_config(&endpoint, config)?;
            blockchain.transaction_broadcast(tx)?;

            // Try insert transactions into wallet (without wait for the next sync)
            let txid: Txid = tx.txid();
            match self
                .manager
                .insert_tx(
                    policy_id,
                    tx.clone(),
                    ConfirmationTime::Unconfirmed {
                        last_seen: Timestamp::now().as_u64(),
                    },
                )
                .await
            {
                Ok(res) => {
                    if res {
                        tracing::debug!("Tx {txid} added into the wallet");
                    } else {
                        tracing::warn!("Tx {txid} not added into the wallet! It will appear in the next policy sync.");
                    }
                }
                Err(e) => tracing::error!("Impossible to insert tx {txid} into wallet: {e}."),
            }
        }

        // Compose the event
        let content: String = completed_proposal.encrypt_with_keys(&shared_keys)?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(proposal_id, None, None));
        tags.push(Tag::Event(policy_id, None, None));
        let event =
            EventBuilder::new(COMPLETED_PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;

        // Publish the event
        let event_id = self.send_event(event).await?;

        // Delete the proposal
        if let Err(e) = self.delete_proposal_by_id(proposal_id).await {
            tracing::error!("Impossibe to delete proposal {proposal_id}: {e}");
        }

        // Cache
        self.db.delete_proposal(proposal_id).await?;
        self.db
            .save_completed_proposal(event_id, policy_id, completed_proposal.clone())
            .await?;

        Ok(completed_proposal)
    }

    pub async fn new_proof_proposal<S>(
        &self,
        policy_id: EventId,
        message: S,
    ) -> Result<(EventId, Proposal, EventId), Error>
    where
        S: Into<String>,
    {
        let message: &str = &message.into();

        // Build proposal
        let proposal: Proposal = self.manager.proof_of_reserve(policy_id, message).await?;

        // Get shared keys
        let shared_keys = self.db.get_shared_key(policy_id).await?;

        // Compose the event
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;
        let mut tags: Vec<Tag> = nostr_pubkeys
            .iter()
            .map(|p| Tag::PubKey(*p, None))
            .collect();
        tags.push(Tag::Event(policy_id, None, None));
        let content = proposal.encrypt_with_keys(&shared_keys)?;
        // Publish proposal with `shared_key` so every owner can delete it
        let event = EventBuilder::new(PROPOSAL_KIND, content, &tags).to_event(&shared_keys)?;
        let proposal_id = self.send_event(event).await?;

        // Send DM msg
        // TODO: send withoud wait for OK
        /* let sender = self.client.keys().public_key();
        let mut msg = String::from("New Proof of Reserve request:\n");
        msg.push_str(&format!("- Message: {message}"));
        for pubkey in nostr_pubkeys.into_iter() {
            if sender != pubkey {
                self.client.send_direct_msg(pubkey, &msg, None).await?;
            }
        } */

        // Cache proposal
        self.db
            .save_proposal(proposal_id, policy_id, proposal.clone())
            .await?;

        Ok((proposal_id, proposal, policy_id))
    }

    pub async fn verify_proof_by_id(&self, completed_proposal_id: EventId) -> Result<u64, Error> {
        let GetCompletedProposal {
            proposal,
            policy_id,
            ..
        } = self
            .get_completed_proposal_by_id(completed_proposal_id)
            .await?;
        if let CompletedProposal::ProofOfReserve { message, psbt, .. } = proposal {
            Ok(self.manager.verify_proof(policy_id, &psbt, message).await?)
        } else {
            Err(Error::UnexpectedProposal)
        }
    }

    #[deprecated]
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_balance(&self, policy_id: EventId) -> Option<Balance> {
        self.manager.get_balance(policy_id).await.ok()
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_txs(
        &self,
        policy_id: EventId,
        sort: bool,
    ) -> Result<Vec<GetTransaction>, Error> {
        let wallet = self.manager.wallet(policy_id).await?;
        let mut txs = wallet.get_txs().await;

        if sort {
            txs.sort_by(|a, b| {
                let a = match a.confirmation_time {
                    ConfirmationTime::Confirmed { height, .. } => height,
                    ConfirmationTime::Unconfirmed { .. } => u32::MAX,
                };

                let b = match b.confirmation_time {
                    ConfirmationTime::Confirmed { height, .. } => height,
                    ConfirmationTime::Unconfirmed { .. } => u32::MAX,
                };

                b.cmp(&a)
            });
        }

        let descriptions: HashMap<Txid, String> = self.db.get_txs_descriptions(policy_id).await?;
        let script_labels: HashMap<ScriptBuf, Label> =
            self.db.get_addresses_labels(policy_id).await?;

        let block_explorer = self.config.block_explorer().await.ok();

        let mut list: Vec<GetTransaction> = Vec::new();

        for tx in txs.into_iter() {
            let txid: Txid = tx.txid();

            let label: Option<String> = if tx.received > tx.sent {
                let mut label = None;
                for txout in tx.output.iter() {
                    if wallet.is_mine(&txout.script_pubkey).await {
                        label = script_labels.get(&txout.script_pubkey).map(|l| l.text());
                        break;
                    }
                }
                label
            } else {
                // TODO: try to get UTXO label?
                descriptions.get(&txid).cloned()
            };

            list.push(GetTransaction {
                policy_id,
                label,
                tx,
                block_explorer: block_explorer
                    .as_ref()
                    .map(|url| format!("{url}/tx/{txid}")),
            })
        }

        Ok(list)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_tx(&self, policy_id: EventId, txid: Txid) -> Result<GetTransaction, Error> {
        let wallet = self.manager.wallet(policy_id).await?;
        let tx = wallet.get_tx(txid).await?;

        let label: Option<String> = if tx.received > tx.sent {
            let mut label = None;
            for txout in tx.output.iter() {
                if wallet.is_mine(&txout.script_pubkey).await {
                    let shared_key = self.db.get_shared_key(policy_id).await?;
                    let address = Address::from_script(&txout.script_pubkey, self.network)?;
                    let identifier: String =
                        LabelData::Address(Address::new(self.network, address.payload))
                            .generate_identifier(&shared_key)?;
                    label = self
                        .db
                        .get_label_by_identifier(identifier)
                        .await
                        .ok()
                        .map(|l| l.text());
                    break;
                }
            }
            label
        } else {
            // TODO: try to get UTXO label?
            self.db.get_description_by_txid(policy_id, txid).await?
        };

        let block_explorer = self.config.block_explorer().await.ok();

        Ok(GetTransaction {
            policy_id,
            tx,
            label,
            block_explorer: block_explorer
                .as_ref()
                .map(|url| format!("{url}/tx/{txid}")),
        })
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_address(
        &self,
        policy_id: EventId,
        index: AddressIndex,
    ) -> Result<GetAddress, Error> {
        let address = self.manager.get_address(policy_id, index).await?.address;

        let shared_key = self.db.get_shared_key(policy_id).await?;
        let address = Address::new(self.network, address.payload);
        let identifier: String =
            LabelData::Address(address.clone()).generate_identifier(&shared_key)?;
        let label = self
            .db
            .get_label_by_identifier(identifier)
            .await
            .ok()
            .map(|l| l.text());
        Ok(GetAddress { address, label })
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_last_unused_address(&self, policy_id: EventId) -> Result<GetAddress, Error> {
        self.get_address(policy_id, AddressIndex::LastUnused).await
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_addresses(&self, policy_id: EventId) -> Result<Vec<GetAddress>, Error> {
        let script_labels: HashMap<ScriptBuf, Label> =
            self.db.get_addresses_labels(policy_id).await?;
        Ok(self
            .manager
            .get_addresses(policy_id)
            .await?
            .into_iter()
            .map(|address| GetAddress {
                label: script_labels
                    .get(&address.payload.script_pubkey())
                    .map(|l| l.text()),
                address,
            })
            .collect())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_addresses_balances(
        &self,
        policy_id: EventId,
    ) -> Result<HashMap<ScriptBuf, u64>, Error> {
        Ok(self.manager.get_addresses_balances(policy_id).await?)
    }

    /// Get wallet UTXOs
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_utxos(&self, policy_id: EventId) -> Result<Vec<GetUtxo>, Error> {
        // Get labels
        let script_labels: HashMap<ScriptBuf, Label> =
            self.db.get_addresses_labels(policy_id).await?;
        let utxo_labels: HashMap<OutPoint, Label> = self.db.get_utxos_labels(policy_id).await?;
        let frozen_utxos: HashSet<Sha256Hash> = self.db.get_frozen_utxos(policy_id).await?;

        // Compose output
        Ok(self
            .manager
            .get_utxos(policy_id)
            .await?
            .into_iter()
            .map(|utxo| GetUtxo {
                label: utxo_labels
                    .get(&utxo.outpoint)
                    .or_else(|| script_labels.get(&utxo.txout.script_pubkey))
                    .map(|l| l.text()),
                frozen: {
                    let hash = Sha256Hash::hash(utxo.outpoint.to_string().as_bytes());
                    frozen_utxos.contains(&hash)
                },
                utxo,
            })
            .collect())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_total_balance(&self) -> Result<Balance, Error> {
        let mut total_balance = Balance::default();
        let mut already_seen = Vec::new();
        for InternalGetPolicy {
            policy_id, policy, ..
        } in self.db.get_policies().await?.into_iter()
        {
            if !already_seen.contains(&policy.descriptor) {
                let balance = self.manager.get_balance(policy_id).await?;
                total_balance = total_balance.add(balance);
                already_seen.push(policy.descriptor);
            }
        }
        Ok(total_balance)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_all_transactions(&self) -> Result<Vec<GetTransaction>, Error> {
        let mut txs = Vec::new();
        let mut already_seen = Vec::new();
        for InternalGetPolicy {
            policy_id, policy, ..
        } in self.db.get_policies().await?.into_iter()
        {
            if !already_seen.contains(&policy.descriptor) {
                for tx in self
                    .get_txs(policy_id, false)
                    .await
                    .unwrap_or_default()
                    .into_iter()
                {
                    txs.push(tx)
                }
                already_seen.push(policy.descriptor);
            }
        }

        txs.sort_by(|a, b| {
            let a = match a.confirmation_time {
                ConfirmationTime::Confirmed { height, .. } => height,
                ConfirmationTime::Unconfirmed { .. } => u32::MAX,
            };

            let b = match b.confirmation_time {
                ConfirmationTime::Confirmed { height, .. } => height,
                ConfirmationTime::Unconfirmed { .. } => u32::MAX,
            };

            b.cmp(&a)
        });

        Ok(txs)
    }

    pub async fn rebroadcast_all_events(&self) -> Result<(), Error> {
        let pool = self.client.pool();
        let events: Vec<Event> = self.db.get_events().await?;
        for event in events.into_iter() {
            pool.send_msg(ClientMessage::new_event(event), None).await?;
        }
        // TODO: save last rebroadcast timestamp
        Ok(())
    }

    pub async fn rebroadcast_to<S>(&self, url: S) -> Result<(), Error>
    where
        S: Into<String>,
    {
        let url: String = url.into();
        let pool = self.client.pool();
        let events: Vec<Event> = self.db.get_events().await?;
        for event in events.into_iter() {
            pool.send_msg_to(&*url, ClientMessage::new_event(event), None)
                .await?;
        }
        // TODO: save last rebroadcast timestamp
        Ok(())
    }

    pub async fn republish_shared_key_for_policy(&self, policy_id: EventId) -> Result<(), Error> {
        let keys: Keys = self.keys().await;
        let shared_key = self.db.get_shared_key(policy_id).await?;
        let pubkeys = self.db.get_nostr_pubkeys(policy_id).await?;
        // Publish the shared key
        for pubkey in pubkeys.iter() {
            let encrypted_shared_key = nips::nip04::encrypt(
                &keys.secret_key()?,
                pubkey,
                shared_key.secret_key()?.display_secret().to_string(),
            )?;
            let event: Event = EventBuilder::new(
                SHARED_KEY_KIND,
                encrypted_shared_key,
                &[
                    Tag::Event(policy_id, None, None),
                    Tag::PubKey(*pubkey, None),
                ],
            )
            .to_event(&keys)?;
            let event_id: EventId = event.id;

            // TODO: use send_batch_event method from nostr-sdk
            self.db.save_event(event.clone()).await?;
            self.client
                .pool()
                .send_msg(ClientMessage::new_event(event), None)
                .await?;
            tracing::info!("Published shared key for {pubkey} at event {event_id}");
        }
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn export_policy_backup(&self, policy_id: EventId) -> Result<PolicyBackup, Error> {
        let InternalGetPolicy { policy, .. } = self.db.get_policy(policy_id).await?;
        let nostr_pubkeys: Vec<XOnlyPublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;
        Ok(PolicyBackup::new(
            policy.name,
            policy.description,
            policy.descriptor,
            nostr_pubkeys,
        ))
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn save_policy_backup<P>(&self, policy_id: EventId, path: P) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let backup = self.export_policy_backup(policy_id).await?;
        backup.save(path)?;
        Ok(())
    }

    pub async fn get_known_public_keys_with_metadata(&self) -> Result<Vec<User>, Error> {
        Ok(self
            .db
            .get_known_public_keys_with_metadata()
            .await?
            .into_iter()
            .map(|(public_key, metadata)| User::new(public_key, metadata))
            .collect())
    }
}
