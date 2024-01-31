// Copyright (c) 2022-2024 Smart Vaults
// Distributed under the MIT software license

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
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
use nostr_sdk::database::{NostrDatabaseExt, Order};
use nostr_sdk::pool::pool;
use nostr_sdk::prelude::*;
use parking_lot::RwLock as ParkingLotRwLock;
use smartvaults_core::bdk::chain::ConfirmationTime;
use smartvaults_core::bdk::wallet::{AddressIndex, Balance};
use smartvaults_core::bdk::FeeRate as BdkFeeRate;
use smartvaults_core::bips::bip39::Mnemonic;
use smartvaults_core::bitcoin::address::NetworkChecked;
use smartvaults_core::bitcoin::bip32::Fingerprint;
use smartvaults_core::bitcoin::{Address, Network, OutPoint, ScriptBuf, Transaction, Txid};
use smartvaults_core::miniscript::Descriptor;
use smartvaults_core::types::{KeeChain, Keychain, Seed, WordCount};
use smartvaults_core::{Destination, FeeRate, SpendingProposal, SECP256K1};
use smartvaults_protocol::v1::{Label, LabelData};
use smartvaults_protocol::v2::{
    self, Approval, PendingProposal, Proposal, ProposalIdentifier, Signer, VaultIdentifier,
};
use smartvaults_sdk_sqlite::Store;
use tokio::sync::broadcast::{self, Sender};

mod connect;
mod key_agent;
mod label;
mod proposal;
mod signers;
mod sync;
mod vault;

pub use self::sync::{EventHandled, Message};
use crate::config::{Config, ElectrumEndpoint};
use crate::constants::{MAINNET_RELAYS, SEND_TIMEOUT, TESTNET_RELAYS};
use crate::manager::{Manager, SmartVaultsWallet, TransactionDetails};
use crate::storage::{InternalApproval, InternalVault, SmartVaultsStorage};
use crate::types::{
    GetAddress, GetApproval, GetApprovedProposals, GetTransaction, GetUtxo, PolicyBackup,
};
use crate::{util, Error};

/// Smart Vaults Client
#[derive(Debug, Clone)]
pub struct SmartVaults {
    network: Network,
    keechain: Arc<ParkingLotRwLock<KeeChain>>,
    keys: Keys,
    client: Client,
    manager: Manager,
    config: Config,
    storage: SmartVaultsStorage,
    db: Store,
    syncing: Arc<AtomicBool>,
    resubscribe_vaults: Arc<AtomicBool>,
    sync_channel: Sender<Message>,
    default_signer: Signer,
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
        )
        .await?;

        // Nostr client
        let nostr_db_path = util::dir::nostr_db(base_path, keys.public_key(), network)?;
        let nostr_db = SQLiteDatabase::open(nostr_db_path).await?;
        let opts = Options::new()
            .wait_for_send(true)
            .wait_for_subscription(false)
            .skip_disconnected_relays(true)
            .send_timeout(Some(SEND_TIMEOUT));
        let client: Client = ClientBuilder::new()
            .signer(&keys)
            .database(nostr_db)
            .opts(opts)
            .build();

        // Storage
        let storage = SmartVaultsStorage::build(keys.clone(), client.database(), network).await?;

        let (sender, _) = broadcast::channel::<Message>(4096);

        let this = Self {
            network,
            keechain: Arc::new(ParkingLotRwLock::new(keechain)),
            keys,
            client,
            manager: Manager::new(db.clone(), network),
            config: Config::try_from_file(base_path, network)?,
            storage,
            db,
            syncing: Arc::new(AtomicBool::new(false)),
            resubscribe_vaults: Arc::new(AtomicBool::new(false)),
            sync_channel: sender,
            default_signer: Signer::smartvaults(&seed, network)?,
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
        for (vault_id, vault) in self.storage.vaults().await.into_iter() {
            let manager = self.manager.clone();
            thread::spawn(async move {
                if let Err(e) = manager.load_policy(vault_id, vault.policy()).await {
                    tracing::error!("Impossible to load vault {vault_id}: {e}");
                }
            })?;
        }
        self.restore_relays().await?;
        self.client.connect().await;
        self.sync()?;
        Ok(())
    }

    async fn blockchain(&self) -> Result<ElectrumClient, Error> {
        let endpoint = self.config.electrum_endpoint().await?;
        let proxy: Option<SocketAddr> = self.config.proxy().await.ok();
        let config = ElectrumConfig::builder()
            .validate_domain(endpoint.validate_tls())
            .socks5(proxy.map(Socks5Config::new))
            .build();
        Ok(ElectrumClient::from_config(
            &endpoint.as_non_standard_format(),
            config,
        )?)
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
        if let Err(e) = self.sync() {
            tracing::error!("Impossible to start sync: {e}");
        }
    }

    pub async fn stop(&self) -> Result<(), Error> {
        self.client.stop().await?;
        Ok(())
    }

    /// Force a full timechain sync
    pub async fn force_full_timechain_sync(&self) -> Result<(), Error> {
        let endpoint = self.config.electrum_endpoint().await?;
        let proxy = self.config.proxy().await.ok();
        self.manager
            .full_sync_all(endpoint, proxy, true, None)
            .await?;
        Ok(())
    }

    /// Clear cache
    pub async fn clear_cache(&self) -> Result<(), Error> {
        let mut notifications = self.client.notifications();

        // Stop client
        self.client.stop().await?;

        // Wait for stop notification: clear databases and unload policies
        while let Ok(notification) = notifications.recv().await {
            if let RelayPoolNotification::Stop = notification {
                self.db.wipe().await?;
                self.manager.unload_policies().await;
                self.client.database().wipe().await?;
                break;
            }
        }

        // Re-init everything
        self.init().await?;
        Ok(())
    }

    pub fn keychain<T>(&self, password: T) -> Result<Keychain, Error>
    where
        T: AsRef<[u8]>,
    {
        Ok(self.keechain.read().keychain(password)?)
    }

    pub fn keys(&self) -> &Keys {
        &self.keys
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

        let opts = RelayOptions::new().proxy(proxy);

        if self.client.add_relay_with_opts(url.as_str(), opts).await? {
            let relay = self.client.relay(&url).await?;
            let last_sync: Timestamp = match self.db.get_last_relay_sync(url.clone()).await {
                Ok(ts) => ts,
                Err(_) => Timestamp::from(0),
            };
            let filters: Vec<Filter> = self.sync_filters(last_sync).await;
            relay
                .subscribe(
                    filters,
                    RelaySendOptions::new().skip_send_confirmation(true),
                )
                .await?;
            relay.connect(None).await;

            if save_to_relay_list {
                let this = self.clone();
                thread::spawn(async move {
                    if let Err(e) = this.save_relay_list().await {
                        tracing::error!("Impossible to save relay list: {e}");
                    }
                })?;
            }

            if let Err(e) = self.rebroadcast_to(url.clone()).await {
                tracing::error!("Impossible to rebroadcast events to {url}: {e}");
            }
        }

        Ok(())
    }

    /// Save relay list (NIP65)
    pub async fn save_relay_list(&self) -> Result<EventId, Error> {
        let relays = self.client.relays().await;
        let list = relays
            .into_keys()
            .map(|url| (UncheckedUrl::from(url), None));
        let event = EventBuilder::relay_list(list);
        Ok(self.client.send_event_builder(event).await?)
    }

    /// Get default relays for current [`Network`]
    pub fn default_relays(&self) -> Vec<String> {
        match self.network {
            Network::Bitcoin => MAINNET_RELAYS.into_iter().map(|r| r.to_string()).collect(),
            _ => TESTNET_RELAYS.into_iter().map(|r| r.to_string()).collect(),
        }
    }

    async fn load_nostr_connect_relays(&self) -> Result<(), Error> {
        let relays: Vec<Url> = self.db.get_nostr_connect_sessions_relays().await?;
        self.client.add_relays(relays).await?;
        Ok(())
    }

    /// Restore relays
    #[tracing::instrument(skip_all, level = "trace")]
    async fn restore_relays(&self) -> Result<(), Error> {
        let relays = self.db.get_relays(true).await?;
        for (url, proxy) in relays.into_iter() {
            let opts = RelayOptions::new().proxy(proxy);
            self.client.add_relay_with_opts(url, opts).await?;
        }

        if self.client.relays().await.is_empty() {
            for url in self.default_relays().into_iter() {
                let url = Url::parse(&url)?;
                self.client.add_relay(&url).await?;
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

    /// Get config
    pub fn config(&self) -> Config {
        self.config.clone()
    }

    pub async fn set_electrum_endpoint<S>(&self, endpoint: S) -> Result<(), Error>
    where
        S: AsRef<str>,
    {
        // Set electrum endpoint
        self.config.set_electrum_endpoint(Some(endpoint)).await?;
        // Save config file
        self.config.save().await?;
        Ok(())
    }

    pub async fn electrum_endpoint(&self) -> Result<ElectrumEndpoint, Error> {
        Ok(self.config.electrum_endpoint().await?)
    }

    pub fn block_height(&self) -> u32 {
        self.manager.block_height()
    }

    pub async fn set_metadata(&self, metadata: &Metadata) -> Result<(), Error> {
        let builder = EventBuilder::metadata(metadata);
        self.client.send_event_builder(builder).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_profile(&self) -> Result<Profile, Error> {
        let public_key: PublicKey = self.keys().public_key();
        Ok(self.client.database().profile(public_key).await?)
    }

    /// Get [`Metadata`] of [`PublicKey`]
    ///
    /// If not exists in local database, will return an empty [`Metadata`] struct and will request
    /// it to relays
    pub async fn get_public_key_metadata(&self, public_key: PublicKey) -> Result<Metadata, Error> {
        let profile: Profile = self.client.database().profile(public_key).await?;
        let metadata: Metadata = profile.metadata();
        if metadata == Metadata::default() {
            self.client
                .req_events_of(
                    vec![Filter::new()
                        .author(public_key)
                        .kind(Kind::Metadata)
                        .limit(1)],
                    Some(Duration::from_secs(60)),
                )
                .await;
        }
        Ok(metadata)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_contacts(&self) -> Result<BTreeSet<Profile>, Error> {
        let keys = self.keys();
        Ok(self.client.database().contacts(keys.public_key()).await?)
    }

    pub async fn add_contact(&self, public_key: PublicKey) -> Result<(), Error> {
        let keys: &Keys = self.keys();
        if public_key != keys.public_key() {
            // Add contact
            let mut contacts: Vec<Contact> = self
                .client
                .database()
                .contacts_public_keys(keys.public_key())
                .await?
                .into_iter()
                .map(|p| Contact::new::<String>(p, None, None))
                .collect();
            contacts.push(Contact::new::<String>(public_key, None, None));
            let event = EventBuilder::contact_list(contacts);
            self.client.send_event_builder(event).await?;

            // Request contact metadata
            self.client
                .req_events_of(
                    vec![Filter::new()
                        .author(public_key)
                        .kind(Kind::Metadata)
                        .limit(1)],
                    Some(Duration::from_secs(60)),
                )
                .await;
        }

        Ok(())
    }

    pub async fn remove_contact(&self, public_key: PublicKey) -> Result<(), Error> {
        let keys: &Keys = self.keys();
        let contacts: Vec<Contact> = self
            .client
            .database()
            .contacts_public_keys(keys.public_key())
            .await?
            .into_iter()
            .filter(|p| p != &public_key)
            .map(|p| Contact::new::<String>(p, None, None))
            .collect();
        let event = EventBuilder::contact_list(contacts);
        self.client.send_event_builder(event).await?;
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_approvals_by_proposal_id(
        &self,
        proposal_id: ProposalIdentifier,
    ) -> Result<Vec<GetApproval>, Error> {
        let mut list = Vec::new();
        let approvals = self.storage.approvals().await;
        for (
            approval_id,
            InternalApproval {
                public_key,
                approval,
                timestamp,
                ..
            },
        ) in approvals
            .into_iter()
            .filter(|(_, i)| i.approval.proposal_id() == proposal_id)
        {
            list.push(GetApproval {
                approval_id,
                user: self.client.database().profile(public_key).await?,
                approval,
                timestamp,
            });
        }
        list.sort();
        Ok(list)
    }

    // pub async fn get_members_of_policy(&self, policy_id: EventId) -> Result<Vec<Profile>, Error> {
    // let InternalVault { public_keys, .. } = self.storage.vault(&policy_id).await?;
    // let mut users = Vec::with_capacity(public_keys.len());
    // for public_key in public_keys.into_iter() {
    // let metadata = self.get_public_key_metadata(public_key).await?;
    // let user = Profile::new(public_key, metadata);
    // users.push(user);
    // }
    // Ok(users)
    // }

    pub async fn estimate_tx_vsize(
        &self,
        vault_id: &VaultIdentifier,
        destination: &Destination,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
        skip_frozen_utxos: bool,
    ) -> Result<usize, Error> {
        let fee_rate = FeeRate::min_relay_fee();
        let SpendingProposal { psbt, .. } = self
            .internal_spend(
                vault_id,
                destination,
                fee_rate,
                utxos,
                policy_path,
                skip_frozen_utxos,
            )
            .await?;
        Ok(psbt.unsigned_tx.vsize())
    }

    async fn internal_spend(
        &self,
        vault_id: &VaultIdentifier,
        destination: &Destination,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
        skip_frozen_utxos: bool,
    ) -> Result<SpendingProposal, Error> {
        // Check and calculate fee rate
        if !fee_rate.is_valid() {
            return Err(Error::InvalidFeeRate);
        }

        let fee_rate: BdkFeeRate = match fee_rate {
            FeeRate::Priority(priority) => {
                let blockchain = self.blockchain().await?;
                let btc_per_kvb: f32 =
                    blockchain.estimate_fee(priority.target_blocks() as usize)? as f32;
                BdkFeeRate::from_btc_per_kvb(btc_per_kvb)
            }
            FeeRate::Rate(rate) => BdkFeeRate::from_sat_per_vb(rate),
        };

        let mut frozen_utxos: Option<Vec<OutPoint>> = None;
        if !skip_frozen_utxos {
            let set: HashSet<OutPoint> = self.storage.get_frozen_utxos(vault_id).await;
            frozen_utxos = Some(
                self.manager
                    .get_utxos(vault_id)
                    .await?
                    .into_iter()
                    .filter(|utxo| set.contains(&utxo.outpoint))
                    .map(|utxo| utxo.outpoint)
                    .collect(),
            );
        }

        // Build spending proposal
        Ok(self
            .manager
            .spend(
                vault_id,
                destination,
                fee_rate,
                utxos,
                frozen_utxos,
                policy_path,
            )
            .await?)
    }

    /// Make a spending proposal
    pub async fn spend<S>(
        &self,
        vault_id: &VaultIdentifier,
        destination: Destination,
        description: S,
        fee_rate: FeeRate,
        utxos: Option<Vec<OutPoint>>,
        policy_path: Option<BTreeMap<String, Vec<usize>>>,
        skip_frozen_utxos: bool,
    ) -> Result<Proposal, Error>
    where
        S: Into<String>,
    {
        let spending_proposal: SpendingProposal = self
            .internal_spend(
                vault_id,
                &destination,
                fee_rate,
                utxos,
                policy_path,
                skip_frozen_utxos,
            )
            .await?;
        let pending = PendingProposal::Spending {
            descriptor: spending_proposal.descriptor,
            destination,
            description: description.into(),
            psbt: spending_proposal.psbt,
        };
        let proposal = Proposal::pending(*vault_id, pending, self.network);

        // Get vault
        let vault = self.storage.vault(vault_id).await?;

        // Compose and send event
        let event: Event = v2::proposal::build_event(&vault, &proposal)?;
        self.client.send_event(event).await?;

        // Index proposal
        self.storage
            .save_proposal(proposal.compute_id(), proposal.clone())
            .await;

        Ok(proposal)
    }

    // /// Spend to another [`Policy`]
    // pub async fn self_transfer(
    // &self,
    // from_policy_id: EventId,
    // to_policy_id: EventId,
    // amount: Amount,
    // fee_rate: FeeRate,
    // utxos: Option<Vec<OutPoint>>,
    // policy_path: Option<BTreeMap<String, Vec<usize>>>,
    // skip_frozen_utxos: bool,
    // ) -> Result<GetProposal, Error> {
    // let address = self
    // .get_address(to_policy_id, AddressIndex::New)
    // .await?
    // .address;
    // let description: String = format!(
    // "Self transfer from policy #{} to #{}",
    // util::cut_event_id(from_policy_id),
    // util::cut_event_id(to_policy_id)
    // );
    // self.spend(
    // from_policy_id,
    // destination,
    // description,
    // fee_rate,
    // utxos,
    // policy_path,
    // skip_frozen_utxos,
    // )
    // .await
    // }

    pub async fn approve<T>(
        &self,
        proposal_id: &ProposalIdentifier,
        password: T,
    ) -> Result<Approval, Error>
    where
        T: AsRef<[u8]>,
    {
        // Get proposal and policy
        let proposal: Proposal = self.storage.proposal(proposal_id).await?;
        let vault = self.storage.vault(&proposal.vault_id()).await?;

        // Sign PSBT
        let seed: Seed = self.keechain.read().seed(password)?;
        let approval: Approval = proposal.approve(&seed)?;
        drop(seed);

        // Compose the event
        let keys: &Keys = self.keys();
        let event = v2::approval::build_event(&vault, &approval, keys)?;
        let timestamp = event.created_at;

        // Publish the event
        let event_id = self.client.send_event(event).await?;

        // Index approved proposal
        self.storage
            .save_approval(
                event_id,
                InternalApproval {
                    public_key: keys.public_key(),
                    approval: approval.clone(),
                    timestamp,
                },
            )
            .await;

        Ok(approval)
    }

    // pub async fn approve_with_signed_psbt(
    // &self,
    // proposal_id: EventId,
    // signed_psbt: PartiallySignedTransaction,
    // ) -> Result<(EventId, ApprovedProposal), Error> {
    // let keys: &Keys = self.keys();
    //
    // Get proposal and policy
    // let GetProposal {
    // policy_id,
    // proposal,
    // ..
    // } = self.get_proposal_by_id(proposal_id).await?;
    //
    // let approved_proposal = proposal.approve_with_signed_psbt(signed_psbt)?;
    //
    // Get shared keys
    // let shared_key: Keys = self.storage.shared_key(&policy_id).await?;
    //
    // Compose the event
    // let content = approved_proposal.encrypt_with_keys(&shared_key)?;
    // let InternalVault { public_keys, .. } = self.storage.vault(&policy_id).await?;
    // let mut tags: Vec<Tag> = public_keys.into_iter().map(Tag::public_key).collect();
    // tags.push(Tag::event(proposal_id));
    // tags.push(Tag::event(policy_id));
    // tags.push(Tag::Expiration(
    // Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
    // ));
    //
    // let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, tags).to_event(keys)?;
    // let timestamp = event.created_at;
    //
    // Publish the event
    // let event_id = self.client.send_event(event).await?;
    //
    // Index approved proposal
    // self.storage
    // .save_approval(
    // event_id,
    // InternalApproval {
    // proposal_id,
    // vault_id,
    // public_key: keys.public_key(),
    // approval: approved_proposal.clone(),
    // timestamp,
    // },
    // )
    // .await;
    //
    // Ok((event_id, approved_proposal))
    // }

    // pub async fn approve_with_hwi_signer(
    // &self,
    // proposal_id: EventId,
    // signer: Signer,
    // ) -> Result<(EventId, ApprovedProposal), Error> {
    // let keys: &Keys = self.keys();
    //
    // Get proposal and policy
    // let GetProposal {
    // policy_id,
    // proposal,
    // ..
    // } = self.get_proposal_by_id(proposal_id)?;
    //
    // let approved_proposal = proposal.approve_with_hwi_signer(signer, self.network)?;
    //
    // Get shared keys
    // let shared_keys: Keys = self.db.get_shared_key(policy_id).await?;
    //
    // Compose the event
    // let content = approved_proposal.encrypt_with_keys(&shared_keys)?;
    // let nostr_pubkeys: Vec<PublicKey> = self.db.get_nostr_pubkeys(policy_id).await?;
    // let mut tags: Vec<Tag> = nostr_pubkeys
    // .into_iter()
    // .map(|p| Tag::PubKey(p, None))
    // .collect();
    // tags.push(Tag::event(proposal_id));
    // tags.push(Tag::event(policy_id));
    // tags.push(Tag::Expiration(
    // Timestamp::now().add(APPROVED_PROPOSAL_EXPIRATION),
    // ));
    //
    // let event = EventBuilder::new(APPROVED_PROPOSAL_KIND, content, &tags).to_event(&keys)?;
    // let timestamp = event.created_at;
    //
    // Publish the event
    // let event_id = self.client.send_event(event).await?;
    //
    // Cache approved proposal
    // self.db.save_approved_proposal(
    // proposal_id,
    // keys.public_key(),
    // event_id,
    // approved_proposal.clone(),
    // timestamp,
    // )?;
    //
    // Ok((event_id, approved_proposal))
    // }

    // pub async fn revoke_approval(&self, approval_id: EventId) -> Result<(), Error> {
    // let event = self.client.database().event_by_id(approval_id).await?;
    // let author = event.author();
    // let keys: &Keys = self.keys();
    // if author == keys.public_key() {
    // let InternalApproval { vault_id, .. } = self.storage.approval(&approval_id).await?;
    //
    // Get nostr pubkeys linked to policyit?;
    // let InternalVault { public_keys, .. } = self.storage.vault(&vault_id).await?;
    //
    // let mut tags: Vec<Tag> = public_keys.into_iter().map(Tag::public_key).collect();
    // tags.push(Tag::event(approval_id));
    //
    // let event = EventBuilder::new(Kind::EventDeletion, "", tags);
    // self.client.send_event_builder(event).await?;
    //
    // self.storage.delete_approval(&approval_id).await;
    //
    // Ok(())
    // } else {
    // Err(Error::TryingToDeleteNotOwnedEvent)
    // }
    // }

    /// Finalize [`Proposal`]
    pub async fn finalize(&self, proposal_id: &ProposalIdentifier) -> Result<Proposal, Error> {
        // Get Proposal, Approvals and vault
        let GetApprovedProposals {
            mut proposal,
            approvals,
        } = self.storage.approvals_by_proposal_id(proposal_id).await?;
        let vault = self.storage.vault(&proposal.vault_id()).await?;

        // Finalize proposal
        proposal.finalize(approvals)?;

        // Broadcast
        if proposal.is_broadcastable() {
            let tx: &Transaction = proposal.tx();

            let blockchain = self.blockchain().await?;
            blockchain.transaction_broadcast(tx)?;

            // Try insert transactions into wallet (without wait for the next sync)
            let txid: Txid = tx.txid();
            match self
                .manager
                .insert_tx(
                    &proposal.vault_id(),
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

        // Compose and publish event
        let event = v2::proposal::build_event(&vault, &proposal)?;
        self.client.send_event(event).await?;

        // Index proposal
        self.storage
            .save_proposal(*proposal_id, proposal.clone())
            .await;

        Ok(proposal)
    }

    // pub async fn new_proof_proposal<S>(
    // &self,
    // vault_id: &VaultIdentifier,
    // message: S,
    // ) -> Result<(EventId, Proposal, EventId), Error>
    // where
    // S: Into<String>,
    // {
    // let message: &str = &message.into();
    //
    // Build proposal
    // let proof_of_reserve: ProofOfReserveProposal =
    // self.manager.proof_of_reserve(vault_id, message).await?;
    //
    // Get shared keys
    // let shared_key: Keys = self.storage.shared_key(&policy_id).await?;
    //
    // Compose the event
    // let InternalVault { public_keys, .. } = self.storage.vault(&policy_id).await?;
    // let mut tags: Vec<Tag> = public_keys.iter().copied().map(Tag::public_key).collect();
    // tags.push(Tag::event(policy_id));
    // let content = proposal.encrypt_with_keys(&shared_key)?;
    // Publish proposal with `shared_key` so every owner can delete it
    // let event = EventBuilder::new(PROPOSAL_KIND, content, tags).to_event(&shared_key)?;
    // let timestamp = event.created_at;
    // let proposal_id = self.client.send_event(event).await?;
    //
    // Index proposal
    // self.storage
    // .save_proposal(
    // proposal_id,
    // InternalProposal {
    // policy_id,
    // proposal: proposal.clone(),
    // timestamp,
    // },
    // )
    // .await;
    //
    // Ok((proposal_id, proposal, policy_id))
    // }

    // pub async fn verify_proof_by_id(&self, completed_proposal_id: EventId) -> Result<u64, Error> {
    // let GetCompletedProposal {
    // proposal,
    // policy_id,
    // ..
    // } = self
    // .get_completed_proposal_by_id(completed_proposal_id)
    // .await?;
    // if let CompletedProposal::ProofOfReserve { message, psbt, .. } = proposal {
    // Ok(self.manager.verify_proof(policy_id, &psbt, message).await?)
    // } else {
    // Err(Error::UnexpectedProposal)
    // }
    // }

    #[deprecated]
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_balance(&self, vault_id: &VaultIdentifier) -> Result<Balance, Error> {
        Ok(self.manager.get_balance(vault_id).await?)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_txs(
        &self,
        vault_id: &VaultIdentifier,
    ) -> Result<BTreeSet<GetTransaction>, Error> {
        let wallet: SmartVaultsWallet = self.manager.wallet(vault_id).await?;
        let txs: BTreeSet<TransactionDetails> = wallet.txs().await;

        let descriptions: HashMap<Txid, String> = self.storage.txs_descriptions(*vault_id).await;
        let script_labels: HashMap<ScriptBuf, Label> =
            self.storage.get_addresses_labels(*vault_id).await;

        let block_explorer = self.config.block_explorer().await.ok();

        let mut list: BTreeSet<GetTransaction> = BTreeSet::new();

        for tx in txs.into_iter() {
            let txid: Txid = tx.txid();

            let label: Option<String> = if tx.received > tx.sent {
                let mut label: Option<String> = None;
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

            list.insert(GetTransaction {
                vault_id: *vault_id,
                label,
                tx,
                block_explorer: block_explorer
                    .as_ref()
                    .map(|url| format!("{url}/tx/{txid}")),
            });
        }

        Ok(list)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_tx(
        &self,
        vault_id: &VaultIdentifier,
        txid: Txid,
    ) -> Result<GetTransaction, Error> {
        let wallet = self.manager.wallet(vault_id).await?;
        let tx = wallet.get_tx(txid).await?;

        let label: Option<String> = if tx.received > tx.sent {
            // let mut label = None;
            let label = None;
            for txout in tx.output.iter() {
                if wallet.is_mine(&txout.script_pubkey).await {
                    // let shared_key: Keys = self.storage.shared_key(&vault_id).await?;
                    // let address = Address::from_script(&txout.script_pubkey, self.network)?;
                    // let identifier: String =
                    // LabelData::Address(Address::new(self.network, address.payload))
                    // .generate_identifier(&shared_key)?;
                    // label = self
                    // .storage
                    // .get_label_by_identifier(identifier)
                    // .await
                    // .ok()
                    // .map(|l| l.text());
                    // break;
                    todo!()
                }
            }
            label
        } else {
            // TODO: try to get UTXO label?
            self.storage.description_by_txid(*vault_id, txid).await
        };

        let block_explorer = self.config.block_explorer().await.ok();

        Ok(GetTransaction {
            vault_id: *vault_id,
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
        vault: &VaultIdentifier,
        index: AddressIndex,
    ) -> Result<GetAddress, Error> {
        let address: Address<NetworkChecked> =
            self.manager.get_address(vault, index).await?.address;

        let vault = self.storage.vault(vault).await?;
        let shared_key = Keys::new(vault.shared_key());
        let address = Address::new(self.network, address.payload);
        let identifier: String =
            LabelData::Address(address.clone()).generate_identifier(&shared_key)?;
        let label = self
            .storage
            .get_label_by_identifier(identifier)
            .await
            .ok()
            .map(|l| l.text());
        Ok(GetAddress { address, label })
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_last_unused_address(
        &self,
        vault_id: &VaultIdentifier,
    ) -> Result<GetAddress, Error> {
        self.get_address(vault_id, AddressIndex::LastUnused).await
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_addresses(
        &self,
        vault_id: &VaultIdentifier,
    ) -> Result<Vec<GetAddress>, Error> {
        let script_labels: HashMap<ScriptBuf, Label> =
            self.storage.get_addresses_labels(*vault_id).await;
        Ok(self
            .manager
            .get_addresses(vault_id)
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
        vault_id: &VaultIdentifier,
    ) -> Result<HashMap<ScriptBuf, u64>, Error> {
        Ok(self.manager.get_addresses_balances(vault_id).await?)
    }

    /// Get wallet UTXOs
    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_utxos(&self, vault_id: &VaultIdentifier) -> Result<Vec<GetUtxo>, Error> {
        // Get labels
        let script_labels: HashMap<ScriptBuf, Label> =
            self.storage.get_addresses_labels(*vault_id).await;
        let utxo_labels: HashMap<OutPoint, Label> = self.storage.get_utxos_labels(*vault_id).await;
        let frozen_utxos: HashSet<OutPoint> = self.storage.get_frozen_utxos(vault_id).await;

        // Compose output
        Ok(self
            .manager
            .get_utxos(vault_id)
            .await?
            .into_iter()
            .map(|utxo| GetUtxo {
                label: utxo_labels
                    .get(&utxo.outpoint)
                    .or_else(|| script_labels.get(&utxo.txout.script_pubkey))
                    .map(|l| l.text()),
                frozen: frozen_utxos.contains(&utxo.outpoint),
                utxo,
            })
            .collect())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_total_balance(&self) -> Result<Balance, Error> {
        let vaults: HashMap<VaultIdentifier, InternalVault> = self.storage.vaults().await;
        let mut total_balance: Balance = Balance::default();
        #[allow(clippy::mutable_key_type)]
        let mut already_seen: HashSet<Descriptor<String>> = HashSet::with_capacity(vaults.len());
        for (vault_id, vault) in vaults.into_iter() {
            if already_seen.insert(vault.descriptor()) {
                let balance: Balance = self.manager.get_balance(&vault_id).await?;
                total_balance = total_balance.add(balance);
            }
        }
        Ok(total_balance)
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn get_all_transactions(&self) -> Result<BTreeSet<GetTransaction>, Error> {
        let vaults: HashMap<VaultIdentifier, InternalVault> = self.storage.vaults().await;
        let mut txs: BTreeSet<GetTransaction> = BTreeSet::new();
        #[allow(clippy::mutable_key_type)]
        let mut already_seen: HashSet<Descriptor<String>> = HashSet::with_capacity(vaults.len());
        for (vault_id, vault) in vaults.into_iter() {
            if already_seen.insert(vault.descriptor()) {
                let t = self.get_txs(&vault_id).await?;
                txs.extend(t);
            }
        }
        Ok(txs)
    }

    pub async fn rebroadcast_all_events(&self) -> Result<(), Error> {
        let pool = self.client.pool();
        let events: Vec<Event> = self
            .client
            .database()
            .query(vec![Filter::new()], Order::Asc)
            .await?;
        for event in events.into_iter() {
            pool.send_msg(
                ClientMessage::event(event),
                RelaySendOptions::new().skip_send_confirmation(true),
            )
            .await?;
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
        let events: Vec<Event> = self
            .client
            .database()
            .query(vec![Filter::new()], Order::Asc)
            .await?;
        for event in events.into_iter() {
            pool.send_msg_to(
                [&*url],
                ClientMessage::event(event),
                RelaySendOptions::new().skip_send_confirmation(true),
            )
            .await?;
        }
        // TODO: save last rebroadcast timestamp
        Ok(())
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn export_policy_backup(
        &self,
        vault_id: &VaultIdentifier,
    ) -> Result<PolicyBackup, Error> {
        let vault = self.storage.vault(vault_id).await?;
        Ok(PolicyBackup::new(
            "TODO",
            "TODO",
            vault.descriptor(),
            Vec::new(),
        ))
    }

    #[tracing::instrument(skip_all, level = "trace")]
    pub async fn save_vault_backup<P>(
        &self,
        vault_id: &VaultIdentifier,
        path: P,
    ) -> Result<(), Error>
    where
        P: AsRef<Path>,
    {
        let backup = self.export_policy_backup(vault_id).await?;
        backup.save(path)?;
        Ok(())
    }

    pub async fn get_known_profiles(&self) -> Result<BTreeSet<Profile>, Error> {
        let filter = Filter::new().kind(Kind::Metadata);
        Ok(self
            .client
            .database()
            .query(vec![filter], Order::Desc)
            .await?
            .into_iter()
            .map(|e| {
                let metadata = Metadata::from_json(e.content()).unwrap_or_default();
                Profile::new(e.pubkey, metadata)
            })
            .collect())
    }
}
